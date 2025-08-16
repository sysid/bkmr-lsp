// File: bkmr-lsp/src/backend.rs - Word-based completion with manual triggering

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tower_lsp::{Client, LanguageServer, jsonrpc::Result as LspResult, lsp_types::*};
use tracing::{debug, error, info, instrument, warn};

/// Language-specific information for Rust pattern translation
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub line_comment: Option<String>,
    pub block_comment: Option<(String, String)>,
    pub indent_char: String,
}

/// Configuration for the bkmr-lsp server
#[derive(Debug, Clone)]
pub struct BkmrConfig {
    pub bkmr_binary: String,
    pub max_completions: usize,
}

impl Default for BkmrConfig {
    fn default() -> Self {
        Self {
            bkmr_binary: "bkmr".to_string(),
            max_completions: 50,
        }
    }
}

/// Represents a bkmr snippet from JSON output
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BkmrSnippet {
    pub id: i32,
    pub title: String,
    pub url: String, // This contains the actual snippet content
    pub description: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub access_count: i32,
}

#[derive(Debug)]
pub struct BkmrLspBackend {
    client: Client,
    config: BkmrConfig,
    /// Cache of document contents to extract prefixes
    document_cache: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, String>>>,
    /// Cache of document language IDs for filetype-based filtering
    language_cache: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, String>>>,
}

impl BkmrLspBackend {
    pub fn new(client: Client) -> Self {
        debug!("Creating BkmrLspBackend");
        Self {
            client,
            config: BkmrConfig::default(),
            document_cache: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            language_cache: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }


    /// Extract word backwards from cursor position and return both query and range
    #[instrument(skip(self))]
    fn extract_snippet_query(&self, uri: &Url, position: Position) -> Option<(String, Range)> {
        let cache = self.document_cache.read().ok()?;
        let content = cache.get(&uri.to_string())?;

        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let char_pos = position.character as usize;

        if char_pos > line.len() {
            return None;
        }

        let before_cursor = &line[..char_pos];
        debug!("Extracting from line: '{}', char_pos: {}, before_cursor: '{}'", line, char_pos, before_cursor);
        
        // Extract word backwards from cursor - find where the word starts
        let word_start = before_cursor
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_' || *c == '-')
            .last()
            .map(|(i, _)| i)
            .unwrap_or(char_pos);
        
        debug!("Word boundaries: start={}, end={}", word_start, char_pos);

        if word_start < char_pos {
            let word = &before_cursor[word_start..];
            if !word.is_empty() && word.chars().any(|c| c.is_alphanumeric()) {
                debug!("Extracted word: '{}' from position {}", word, char_pos);
                
                // Create range for the word to be replaced
                let range = Range {
                    start: Position {
                        line: position.line,
                        character: word_start as u32,
                    },
                    end: Position {
                        line: position.line,
                        character: char_pos as u32,
                    },
                };
                
                return Some((word.to_string(), range));
            }
        }

        debug!("No valid word found at position {}", char_pos);
        None
    }

    /// Get the language ID for a document URI
    fn get_language_id(&self, uri: &Url) -> Option<String> {
        let cache = self.language_cache.read().ok()?;
        cache.get(&uri.to_string()).cloned()
    }

    /// Execute bkmr command and return parsed snippets
    #[instrument(skip(self))]
    async fn fetch_snippets(&self, prefix: Option<&str>, language_id: Option<&str>) -> Result<Vec<BkmrSnippet>> {
        let mut args = vec![
            "search".to_string(),
            "--json".to_string(),
            "--interpolate".to_string(), // ← ADD THIS: Always use interpolation
            "--ntags-prefix".to_string(),
            "_snip_".to_string(),
            "--limit".to_string(),
            self.config.max_completions.to_string(),
        ];

        // Add language-based tag filter if available
        if let Some(lang) = language_id {
            if !lang.trim().is_empty() {
                args.push("-t".to_string());
                args.push(lang.to_string());
                debug!("Using language filter: {}", lang);
            }
        }

        // Add search term if prefix is provided and not empty
        if let Some(p) = prefix {
            if !p.trim().is_empty() {
                // Use title prefix search for better snippet matching
                args.push(format!("metadata:{}*", p));
                debug!("Using search prefix: {}", p);
            }
        }

        debug!("Executing bkmr with args: {:?}", args);

        // Add timeout to prevent hanging
        let command_future = tokio::process::Command::new(&self.config.bkmr_binary)
            .args(&args)
            .output();

        let output =
            match tokio::time::timeout(std::time::Duration::from_secs(10), command_future).await {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    error!("Failed to execute bkmr: {}", e);
                    return Err(anyhow!("Failed to execute bkmr: {}", e));
                }
                Err(_) => {
                    error!("bkmr command timed out after 10 seconds");
                    return Err(anyhow!("bkmr command timed out"));
                }
            };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("bkmr command failed with stderr: {}", stderr);
            return Err(anyhow!("bkmr command failed: {}", stderr));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);

        if stdout_str.trim().is_empty() {
            debug!("Empty output from bkmr");
            return Ok(Vec::new());
        }

        let snippets: Vec<BkmrSnippet> = serde_json::from_str(&stdout_str).map_err(|e| {
            error!("Failed to parse bkmr JSON output: {}", e);
            error!("Raw output was: {}", stdout_str);
            anyhow!("Failed to parse bkmr JSON output: {}", e)
        })?;

        info!(
            "Successfully fetched {} interpolated snippets",
            snippets.len()
        );
        Ok(snippets)
    }

    /// Translate Rust syntax patterns in universal snippets to target language
    pub fn translate_rust_patterns(&self, content: &str, language_id: &str, uri: &Url) -> String {
        let target_lang = self.get_language_info(language_id);
        let mut processed_content = content.to_string();

        debug!("Translating Rust patterns for language: {}", language_id);

        // Replace Rust line comments (//) with target language line comments
        if let Some(target_comment) = &target_lang.line_comment {
            // Match // at start of line or after whitespace - use MULTILINE flag
            let line_comment_regex = regex::RegexBuilder::new(r"^(\s*)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = line_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{} {}", &caps[1], target_comment, &caps[2])
            }).to_string();
            
            // Match // at end of line (after code) - use MULTILINE flag
            let eol_comment_regex = regex::RegexBuilder::new(r"(\S+)(\s+)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = eol_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{}{} {}", &caps[1], &caps[2], target_comment, &caps[3])
            }).to_string();
        } else if let Some((block_start, block_end)) = &target_lang.block_comment {
            // For languages without line comments, replace // with block comments
            // Handle start-of-line comments
            let line_comment_regex = regex::RegexBuilder::new(r"^(\s*)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = line_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{} {} {}", &caps[1], block_start, &caps[2], block_end)
            }).to_string();
            
            // Handle end-of-line comments
            let eol_comment_regex = regex::RegexBuilder::new(r"(\S+)(\s+)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = eol_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{}{} {} {}", &caps[1], &caps[2], block_start, &caps[3], block_end)
            }).to_string();
        }

        // Replace Rust block comments (/* */) with target language block comments
        if let Some((target_start, target_end)) = &target_lang.block_comment {
            let block_comment_regex = regex::RegexBuilder::new(r"/\*(.*?)\*/")
                .dot_matches_new_line(true)
                .build()
                .unwrap();
            processed_content = block_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{}{}", target_start, &caps[1], target_end)
            }).to_string();
        }

        // Replace Rust indentation (4 spaces) with target language indentation
        // Detect lines that start with 4 spaces and replace with target indentation
        if target_lang.indent_char != "    " {
            let indent_regex = regex::RegexBuilder::new(r"^( {4})+")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = indent_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                let rust_indent_count = caps[0].len() / 4; // Number of 4-space indents
                target_lang.indent_char.repeat(rust_indent_count)
            }).to_string();
        }

        // Add file name replacement for simple relative path
        if processed_content.contains("{{ filename }}") {
            let filename = uri.path().split('/').last().unwrap_or("untitled");
            processed_content = processed_content.replace("{{ filename }}", filename);
        }

        debug!("Rust pattern translation complete");
        processed_content
    }

    /// Convert bkmr snippet to LSP completion item with proper text replacement
    fn snippet_to_completion_item_with_trigger(
        &self,
        snippet: &BkmrSnippet,
        query: &str,
        replacement_range: Option<Range>,
        language_id: &str,
        uri: &Url,
    ) -> CompletionItem {
        // Check if this is a universal snippet and process accordingly
        let snippet_content = if snippet.tags.contains(&"universal".to_string()) {
            debug!("Processing universal snippet: {}", snippet.title);
            self.translate_rust_patterns(&snippet.url, language_id, uri)
        } else {
            // Regular snippet - use content as-is
            snippet.url.clone()
        };
        let label = snippet.title.clone();

        debug!(
            "Creating completion item: query='{}', label='{}', content_preview='{}'",
            query,
            label,
            snippet_content.chars().take(20).collect::<String>()
        );

        let mut completion_item = CompletionItem {
            label: label.clone(),
            kind: Some(CompletionItemKind::TEXT),  // TODO: Snippet
            detail: Some(format!("bkmr snippet")),
            documentation: Some(Documentation::String(if snippet_content.len() > 500 {
                format!("{}...", &snippet_content[..500])
            } else {
                snippet_content.clone()
            })),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),  // TODO: Snippet
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            ..Default::default()
        };

        // Use TextEdit for proper replacement if we have a range
        if let Some(range) = replacement_range {
            completion_item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: snippet_content,
            }));
            debug!("Set text_edit for range replacement: {:?}", range);
        } else {
            // Fallback to insert_text for backward compatibility
            completion_item.insert_text = Some(snippet_content);
            debug!("Using fallback insert_text (no range available)");
        }

        completion_item
    }

    /// Check if bkmr binary is available
    #[instrument(skip(self))]
    async fn verify_bkmr_availability(&self) -> Result<()> {
        debug!("Verifying bkmr availability");

        let command_future = tokio::process::Command::new(&self.config.bkmr_binary)
            .args(["--help"])
            .output();

        let output =
            match tokio::time::timeout(std::time::Duration::from_secs(5), command_future).await {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    return Err(anyhow!("bkmr binary not found: {}", e));
                }
                Err(_) => {
                    return Err(anyhow!("bkmr --help command timed out"));
                }
            };

        if !output.status.success() {
            return Err(anyhow!("bkmr binary is not working properly"));
        }

        info!("bkmr binary verified successfully");
        Ok(())
    }

    /// Get language-specific information for Rust pattern translation
    pub fn get_language_info(&self, language_id: &str) -> LanguageInfo {
        match language_id.to_lowercase().as_str() {
            "rust" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "javascript" | "js" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "  ".to_string(),
            },
            "typescript" | "ts" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "  ".to_string(),
            },
            "python" => LanguageInfo {
                line_comment: Some("#".to_string()),
                block_comment: Some(("\"\"\"".to_string(), "\"\"\"".to_string())),
                indent_char: "    ".to_string(),
            },
            "go" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "\t".to_string(),
            },
            "java" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "c" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "cpp" | "c++" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "html" => LanguageInfo {
                line_comment: None,
                block_comment: Some(("<!--".to_string(), "-->".to_string())),
                indent_char: "  ".to_string(),
            },
            "css" => LanguageInfo {
                line_comment: None,
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "  ".to_string(),
            },
            "scss" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "  ".to_string(),
            },
            "ruby" => LanguageInfo {
                line_comment: Some("#".to_string()),
                block_comment: Some(("=begin".to_string(), "=end".to_string())),
                indent_char: "  ".to_string(),
            },
            "php" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "swift" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "kotlin" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "shell" | "bash" | "sh" => LanguageInfo {
                line_comment: Some("#".to_string()),
                block_comment: None,
                indent_char: "    ".to_string(),
            },
            "yaml" | "yml" => LanguageInfo {
                line_comment: Some("#".to_string()),
                block_comment: None,
                indent_char: "  ".to_string(),
            },
            "json" => LanguageInfo {
                line_comment: None,
                block_comment: None,
                indent_char: "  ".to_string(),
            },
            "markdown" | "md" => LanguageInfo {
                line_comment: None,
                block_comment: Some(("<!--".to_string(), "-->".to_string())),
                indent_char: "  ".to_string(),
            },
            "xml" => LanguageInfo {
                line_comment: None,
                block_comment: Some(("<!--".to_string(), "-->".to_string())),
                indent_char: "  ".to_string(),
            },
            "vim" | "viml" => LanguageInfo {
                line_comment: Some("\"".to_string()),
                block_comment: None,
                indent_char: "  ".to_string(),
            },
            // Default fallback for unknown languages
            _ => LanguageInfo {
                line_comment: Some("#".to_string()),
                block_comment: None,
                indent_char: "    ".to_string(),
            },
        }
    }

    /// Test-only method to create language info without backend instance
    #[cfg(test)]
    pub fn get_language_info_static(language_id: &str) -> LanguageInfo {
        match language_id.to_lowercase().as_str() {
            "rust" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "    ".to_string(),
            },
            "python" => LanguageInfo {
                line_comment: Some("#".to_string()),
                block_comment: Some(("\"\"\"".to_string(), "\"\"\"".to_string())),
                indent_char: "    ".to_string(),
            },
            "javascript" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "  ".to_string(),
            },
            "go" => LanguageInfo {
                line_comment: Some("//".to_string()),
                block_comment: Some(("/*".to_string(), "*/".to_string())),
                indent_char: "\t".to_string(),
            },
            "html" => LanguageInfo {
                line_comment: None,
                block_comment: Some(("<!--".to_string(), "-->".to_string())),
                indent_char: "  ".to_string(),
            },
            _ => LanguageInfo {
                line_comment: Some("#".to_string()),
                block_comment: None,
                indent_char: "    ".to_string(),
            },
        }
    }

    /// Test-only method to translate Rust patterns without backend instance
    #[cfg(test)]
    pub fn translate_rust_patterns_static(content: &str, language_id: &str, uri: &Url) -> String {
        let target_lang = Self::get_language_info_static(language_id);
        let mut processed_content = content.to_string();

        // Replace Rust line comments (//) with target language line comments
        if let Some(target_comment) = &target_lang.line_comment {
            let line_comment_regex = regex::RegexBuilder::new(r"^(\s*)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = line_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{} {}", &caps[1], target_comment, &caps[2])
            }).to_string();
            
            let eol_comment_regex = regex::RegexBuilder::new(r"(\S+)(\s+)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = eol_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{}{} {}", &caps[1], &caps[2], target_comment, &caps[3])
            }).to_string();
        } else if let Some((block_start, block_end)) = &target_lang.block_comment {
            // Handle start-of-line comments
            let line_comment_regex = regex::RegexBuilder::new(r"^(\s*)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = line_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{} {} {}", &caps[1], block_start, &caps[2], block_end)
            }).to_string();
            
            // Handle end-of-line comments
            let eol_comment_regex = regex::RegexBuilder::new(r"(\S+)(\s+)//\s*(.*)$")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = eol_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{}{} {} {}", &caps[1], &caps[2], block_start, &caps[3], block_end)
            }).to_string();
        }

        // Replace Rust block comments
        if let Some((target_start, target_end)) = &target_lang.block_comment {
            let block_comment_regex = regex::RegexBuilder::new(r"/\*(.*?)\*/")
                .dot_matches_new_line(true)
                .build()
                .unwrap();
            processed_content = block_comment_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                format!("{}{}{}", target_start, &caps[1], target_end)
            }).to_string();
        }

        // Replace indentation
        if target_lang.indent_char != "    " {
            let indent_regex = regex::RegexBuilder::new(r"^( {4})+")
                .multi_line(true)
                .build()
                .unwrap();
            processed_content = indent_regex.replace_all(&processed_content, |caps: &regex::Captures| {
                let rust_indent_count = caps[0].len() / 4;
                target_lang.indent_char.repeat(rust_indent_count)
            }).to_string();
        }

        // Replace filename
        if processed_content.contains("{{ filename }}") {
            let filename = uri.path().split('/').last().unwrap_or("untitled");
            processed_content = processed_content.replace("{{ filename }}", filename);
        }

        processed_content
    }

    /// Legacy method for backward compatibility - uses new language info system
    fn get_comment_syntax(&self, file_path: &str) -> &'static str {
        let path = Path::new(file_path);
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        
        // Map file extension to language ID for language info lookup
        let language_id = match extension {
            "rs" => "rust",
            "js" | "mjs" => "javascript", 
            "ts" | "tsx" => "typescript",
            "py" | "pyw" => "python",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" => "cpp",
            "html" | "htm" => "html",
            "css" => "css",
            "scss" => "scss",
            "rb" => "ruby",
            "php" => "php",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "sh" | "bash" | "zsh" => "shell",
            "yaml" | "yml" => "yaml",
            "json" => "json",
            "md" | "markdown" => "markdown",
            "xml" => "xml",
            "vim" => "vim",
            _ => "unknown",
        };
        
        let lang_info = self.get_language_info(language_id);
        // Return line comment or block comment start, fallback to #
        if let Some(_line_comment) = &lang_info.line_comment {
            // This is a bit of a hack since we need to return &'static str
            // but the LanguageInfo returns String. For the legacy method,
            // we'll use a simple lookup.
            match language_id {
                "rust" | "javascript" | "typescript" | "go" | "java" | "c" | "cpp" | "swift" | "kotlin" | "scss" | "php" => "//",
                "python" | "shell" | "yaml" => "#",
                "html" | "markdown" | "xml" => "<!--",
                "css" => "/*",
                "vim" => "\"",
                _ => "#",
            }
        } else {
            "#"
        }
    }

    /// Get the relative path from project root
    fn get_relative_path(&self, file_uri: &str) -> String {
        let url = match Url::parse(file_uri) {
            Ok(u) => u,
            Err(_) => return file_uri.to_string(),
        };

        let file_path = match url.to_file_path() {
            Ok(p) => p,
            Err(_) => return file_uri.to_string(),
        };

        // Try to find a project root by looking for common indicators
        let mut current = file_path.as_path();
        while let Some(parent) = current.parent() {
            // Check for common project root indicators
            if parent.join("Cargo.toml").exists()
                || parent.join("package.json").exists()
                || parent.join("pom.xml").exists()
                || parent.join("build.gradle").exists()
                || parent.join("build.gradle.kts").exists()
                || parent.join("Makefile").exists()
                || parent.join(".git").exists()
            {
                // Found project root, return relative path
                if let Ok(rel_path) = file_path.strip_prefix(parent) {
                    return rel_path.to_string_lossy().to_string();
                }
                break;
            }
            current = parent;
        }

        // Fall back to just the filename if no project root found
        file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file_uri.to_string())
    }

    /// Insert filepath comment at the beginning of the file
    #[instrument(skip(self))]
    async fn insert_filepath_comment(&self, file_uri: &str) -> Result<Vec<TextEdit>> {
        let relative_path = self.get_relative_path(file_uri);
        let comment_syntax = self.get_comment_syntax(file_uri);

        let comment_text = match comment_syntax {
            "<!--" => format!("<!-- {} -->\n", relative_path),
            "/*" => format!("/* {} */\n", relative_path),
            _ => format!("{} {}\n", comment_syntax, relative_path),
        };

        debug!("Inserting filepath comment: {}", comment_text.trim());

        // Create a text edit to insert at the beginning of the file
        let edit = TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            new_text: comment_text,
        };

        Ok(vec![edit])
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BkmrLspBackend {
    #[instrument(skip(self, params))]
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        info!(
            "Initialize request received from client: {:?}",
            params.client_info
        );

        // Verify bkmr is available
        if let Err(e) = self.verify_bkmr_availability().await {
            error!("bkmr verification failed: {}", e);
            self.client
                .log_message(
                    MessageType::ERROR,
                    &format!("Failed to verify bkmr availability: {}", e),
                )
                .await;
        }

        // Check if client supports snippets
        let snippet_support = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.completion.as_ref())
            .and_then(|comp| comp.completion_item.as_ref())
            .and_then(|item| item.snippet_support)
            .unwrap_or(false);

        info!("Client snippet support: {}", snippet_support);

        if !snippet_support {
            warn!("Client does not support snippets");
            self.client
                .log_message(
                    MessageType::WARNING,
                    "Client does not support snippets, functionality may be limited",
                )
                .await;
        }

        let result = InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None, // No automatic triggers - manual completion only
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    completion_item: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["bkmr.insertFilepathComment".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        info!("Initialize complete - manual completion only (no trigger characters)");
        Ok(result)
    }

    #[instrument(skip(self))]
    async fn initialized(&self, _: InitializedParams) {
        info!("Server initialized successfully");

        self.client
            .log_message(MessageType::INFO, "bkmr-lsp server ready")
            .await;
    }

    #[instrument(skip(self))]
    async fn shutdown(&self) -> LspResult<()> {
        info!("Shutdown request received");
        self.client
            .log_message(MessageType::INFO, "Shutting down bkmr-lsp server")
            .await;
        Ok(())
    }

    #[instrument(skip(self, params))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = params.text_document.text;
        let language_id = params.text_document.language_id;

        debug!("Document opened: {} (language: {})", uri, language_id);

        if let Ok(mut cache) = self.document_cache.write() {
            cache.insert(uri.clone(), content);
        }

        if let Ok(mut lang_cache) = self.language_cache.write() {
            lang_cache.insert(uri, language_id);
        }
    }

    #[instrument(skip(self, params))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        debug!("Document changed: {}", uri);

        if let Ok(mut cache) = self.document_cache.write() {
            for change in params.content_changes {
                if let Some(content) = cache.get_mut(&uri) {
                    // For FULL sync, replace entire content
                    if change.range.is_none() {
                        *content = change.text;
                    } else {
                        // For incremental sync, would need more complex logic
                        // For now, just replace entirely
                        *content = change.text;
                    }
                }
            }
        }
    }

    #[instrument(skip(self, params))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        debug!("Document closed: {}", uri);

        if let Ok(mut cache) = self.document_cache.write() {
            cache.remove(&uri);
        }

        if let Ok(mut lang_cache) = self.language_cache.write() {
            lang_cache.remove(&uri);
        }
    }

    #[instrument(skip(self, params))]
    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        debug!(
            "Completion request for {}:{},{}",
            uri, position.line, position.character
        );

        // Only respond to manual completion requests (Ctrl+Space)
        if let Some(context) = &params.context {
            match context.trigger_kind {
                CompletionTriggerKind::INVOKED => {
                    // Manual Ctrl+Space - proceed with word-based completion
                    debug!("Manual completion request - proceeding with word-based snippet search");
                }
                CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS => {
                    debug!("Completion for incomplete results - proceeding");
                }
                _ => {
                    debug!("Ignoring automatic trigger - only manual completion supported");
                    return Ok(Some(CompletionResponse::Array(vec![])));
                }
            }
        } else {
            debug!("No completion context - skipping");
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        // Extract the query after trigger and get replacement range
        let query_info = self.extract_snippet_query(uri, position);
        debug!("Extracted snippet query info: {:?}", query_info);

        // Get the language ID for filetype-based filtering
        let language_id = self.get_language_id(uri);
        debug!("Document language ID: {:?}", language_id);

        // Extract query and range information
        let (query_str, replacement_range) = if let Some((query, range)) = query_info {
            debug!("Query: '{}', Range: {:?}", query, range);
            (query, Some(range))
        } else {
            debug!("No query extracted, using empty query");
            (String::new(), None)
        };

        match self.fetch_snippets(if query_str.is_empty() { None } else { Some(&query_str) }, language_id.as_deref()).await {
            Ok(snippets) => {
                let completion_items: Vec<CompletionItem> = snippets
                    .iter()
                    .map(|snippet| {
                        self.snippet_to_completion_item_with_trigger(
                            snippet, 
                            &query_str, 
                            replacement_range.clone(),
                            language_id.as_deref().unwrap_or("unknown"),
                            uri,
                        )
                    })
                    .collect();

                info!(
                    "Returning {} completion items for query: {:?}",
                    completion_items.len(),
                    query_str
                );

                // Only log first few items to reduce noise in LSP logs
                for (i, item) in completion_items.iter().enumerate().take(3) {
                    debug!(
                        "Item {}: label='{}', sort_text={:?}",
                        i, item.label, item.sort_text
                    );
                }
                if completion_items.len() > 3 {
                    debug!("... and {} more items", completion_items.len() - 3);
                }

                Ok(Some(CompletionResponse::List(CompletionList {
                    is_incomplete: true,
                    items: completion_items,
                })))
            }
            Err(e) => {
                error!("Failed to fetch snippets: {}", e);
                self.client
                    .log_message(
                        MessageType::ERROR,
                        &format!("Failed to fetch snippets: {}", e),
                    )
                    .await;
                Ok(Some(CompletionResponse::Array(vec![])))
            }
        }
    }

    #[instrument(skip(self, params))]
    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> LspResult<Option<serde_json::Value>> {
        debug!("Execute command request: {}", params.command);

        match params.command.as_str() {
            "bkmr.insertFilepathComment" => {
                // Extract file URI from arguments
                if !params.arguments.is_empty() {
                    if let Some(first_arg) = params.arguments.first() {
                        if let Ok(uri_str) = serde_json::from_value::<String>(first_arg.clone()) {
                            match self.insert_filepath_comment(&uri_str).await {
                                Ok(edits) => {
                                    // Apply the text edits to the document
                                    let workspace_edit = WorkspaceEdit {
                                        changes: Some({
                                            let mut changes = std::collections::HashMap::new();
                                            if let Ok(uri) = Url::parse(&uri_str) {
                                                changes.insert(uri, edits);
                                            }
                                            changes
                                        }),
                                        document_changes: None,
                                        change_annotations: None,
                                    };

                                    // Request client to apply the edit
                                    match self.client.apply_edit(workspace_edit).await {
                                        Ok(response) => {
                                            if response.applied {
                                                info!("Successfully inserted filepath comment");
                                                self.client
                                                    .log_message(
                                                        MessageType::INFO,
                                                        "Filepath comment inserted successfully",
                                                    )
                                                    .await;
                                            } else {
                                                warn!("Client rejected the edit");
                                                self.client
                                                    .log_message(
                                                        MessageType::WARNING,
                                                        "Failed to apply filepath comment edit",
                                                    )
                                                    .await;
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to apply edit: {}", e);
                                            self.client
                                                .log_message(
                                                    MessageType::ERROR,
                                                    &format!("Failed to apply edit: {}", e),
                                                )
                                                .await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to create filepath comment: {}", e);
                                    self.client
                                        .log_message(
                                            MessageType::ERROR,
                                            &format!("Failed to create filepath comment: {}", e),
                                        )
                                        .await;
                                }
                            }
                        } else {
                            error!("Invalid argument format for insertFilepathComment");
                        }
                    } else {
                        error!("No arguments provided for insertFilepathComment command");
                    }
                } else {
                    error!("No arguments provided for insertFilepathComment command");
                }
            }
            _ => {
                error!("Unknown command: {}", params.command);
                self.client
                    .log_message(
                        MessageType::ERROR,
                        &format!("Unknown command: {}", params.command),
                    )
                    .await;
            }
        }

        Ok(None)
    }
}
