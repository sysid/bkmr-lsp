use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use tower_lsp::lsp_types::Url;
use tracing::{debug, instrument};

use crate::domain::{LanguageInfo, LanguageRegistry, Snippet};

// Pre-compiled regex patterns for performance
lazy_static! {
    static ref LINE_COMMENT_START: Regex =
        Regex::new(r"^(\s*)//\s*(.*)$").expect("compile line comment start regex");
    static ref LINE_COMMENT_END: Regex =
        Regex::new(r"^(.+?)(\s+)//\s*(.*)$").expect("compile line comment end regex");
    static ref RUST_INDENT: Regex =
        Regex::new(r"^( {4})+").expect("compile rust indentation regex");
    // Environment variable patterns that should be escaped
    static ref ENV_VAR_SIMPLE: Regex =
        Regex::new(r"\$([A-Z_][A-Z0-9_]*)").expect("compile simple env var regex");
    static ref ENV_VAR_BRACED: Regex =
        Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").expect("compile braced env var regex");
    // LSP snippet patterns that should NOT be escaped
    static ref LSP_TABSTOP: Regex =
        Regex::new(r"\$(\d+)").expect("compile LSP tabstop regex");
    static ref LSP_PLACEHOLDER: Regex =
        Regex::new(r"\$\{(\d+:[^}]*)\}").expect("compile LSP placeholder regex");
    static ref LSP_CHOICE: Regex =
        Regex::new(r"\$\{(\d+\|[^}]*\|)\}").expect("compile LSP choice regex");
}

/// Service for translating Rust syntax patterns to target languages
pub struct LanguageTranslator;

impl LanguageTranslator {
    /// Translate Rust syntax patterns in universal snippets to target language
    #[instrument(skip(snippet))]
    pub fn translate_snippet(snippet: &Snippet, language_id: &str, uri: &Url) -> Result<String> {
        let content = if snippet.is_universal() {
            debug!("Processing universal snippet: {}", snippet.title);
            debug!("Original content: {:?}", snippet.get_content());

            Self::translate_rust_patterns(snippet.get_content(), language_id, uri)
                .context("translate Rust patterns to target language")?
        } else {
            // Regular snippet - return content as-is
            snippet.get_content().to_string()
        };

        debug!("Final translated content: {:?}", content);
        Ok(content)
    }

    /// Translate Rust syntax patterns in content to target language
    #[instrument(skip(content))]
    pub fn translate_rust_patterns(content: &str, language_id: &str, uri: &Url) -> Result<String> {
        let target_lang = LanguageRegistry::get_language_info(language_id);

        debug!("Translating Rust patterns for language: {}", language_id);
        debug!("Input content: {:?}", content);
        debug!("Content length: {} bytes", content.len());

        // Use line-by-line processing to preserve newlines
        let mut processed_content =
            Self::translate_rust_patterns_line_by_line(content, &target_lang)
                .context("process content line by line")?;

        // Replace Rust block comments (/* */) with target language block comments
        if let Some((target_start, target_end)) = &target_lang.block_comment {
            let block_comment_regex = RegexBuilder::new(r"/\*(.*?)\*/")
                .dot_matches_new_line(true)
                .build()
                .context("compile block comment regex")?;

            processed_content = block_comment_regex
                .replace_all(&processed_content, |caps: &regex::Captures| {
                    format!("{}{}{}", target_start, &caps[1], target_end)
                })
                .to_string();
        }

        // Add file name replacement for simple relative path
        if processed_content.contains("{{ filename }}") {
            let filename = uri.path().split('/').next_back().unwrap_or("untitled");
            processed_content = processed_content.replace("{{ filename }}", filename);
        }

        debug!("Rust pattern translation complete");
        debug!("Final content: {:?}", processed_content);
        debug!("Final content length: {} bytes", processed_content.len());

        Ok(processed_content)
    }

    /// Process content line by line to preserve newlines properly
    fn translate_rust_patterns_line_by_line(
        content: &str,
        target_lang: &LanguageInfo,
    ) -> Result<String> {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut processed_lines = Vec::new();

        for line in lines {
            let mut processed_line = line.to_string();

            // Process line comments (//)
            if let Some(target_comment) = &target_lang.line_comment {
                // Start of line comments
                if let Some(captures) = LINE_COMMENT_START.captures(line) {
                    processed_line = format!("{}{} {}", &captures[1], target_comment, &captures[2]);
                }
                // End of line comments (after code)
                else if let Some(captures) = LINE_COMMENT_END.captures(line) {
                    processed_line = format!(
                        "{}{}{} {}",
                        &captures[1], &captures[2], target_comment, &captures[3]
                    );
                }
            } else if let Some((block_start, block_end)) = &target_lang.block_comment {
                // For languages without line comments, use block comments
                if let Some(captures) = LINE_COMMENT_START.captures(line) {
                    processed_line = format!(
                        "{}{} {} {}",
                        &captures[1], block_start, &captures[2], block_end
                    );
                } else if let Some(captures) = LINE_COMMENT_END.captures(line) {
                    processed_line = format!(
                        "{}{}{} {} {}",
                        &captures[1], &captures[2], block_start, &captures[3], block_end
                    );
                }
            }

            // Process indentation
            if target_lang.indent_char != "    " {
                if let Some(captures) = RUST_INDENT.captures(&processed_line) {
                    let rust_indent_count = captures[0].len() / 4;
                    let new_indent = target_lang.indent_char.repeat(rust_indent_count);
                    processed_line = processed_line.replacen(&captures[0], &new_indent, 1);
                }
            }

            processed_lines.push(processed_line);
        }

        Ok(processed_lines.join("\n"))
    }

    /// Escape environment variables in content while preserving LSP snippet syntax
    #[instrument(skip(content))]
    pub fn escape_environment_variables(content: &str, should_escape: bool) -> String {
        if !should_escape {
            return content.to_string();
        }

        debug!("Escaping environment variables in content");
        debug!("Input content: {:?}", content);

        // First, collect all LSP snippet positions to avoid escaping them
        let mut lsp_positions = Vec::new();
        
        // Collect LSP tabstop positions ($1, $2, etc.)
        for mat in LSP_TABSTOP.find_iter(content) {
            lsp_positions.push((mat.start(), mat.end()));
        }
        
        // Collect LSP placeholder positions (${1:default})
        for mat in LSP_PLACEHOLDER.find_iter(content) {
            lsp_positions.push((mat.start(), mat.end()));
        }
        
        // Collect LSP choice positions (${1|choice1,choice2|})
        for mat in LSP_CHOICE.find_iter(content) {
            lsp_positions.push((mat.start(), mat.end()));
        }

        // Sort positions by start index for efficient checking
        lsp_positions.sort_by_key(|&(start, _)| start);

        let mut result = content.to_string();
        
        // Helper function to check if a position overlaps with any LSP snippet
        let is_lsp_position = |pos: usize| -> bool {
            lsp_positions.iter().any(|&(start, end)| pos >= start && pos < end)
        };

        // Escape simple environment variables ($VAR) that don't overlap with LSP snippets
        result = ENV_VAR_SIMPLE.replace_all(&result, |caps: &regex::Captures| {
            let full_match = caps.get(0).unwrap();
            let match_start = full_match.start();
            
            if is_lsp_position(match_start) {
                // This is part of an LSP snippet, don't escape
                full_match.as_str().to_string()
            } else {
                // This is an environment variable, escape it
                format!("\\${}", &caps[1])
            }
        }).to_string();

        // Escape braced environment variables (${VAR}) that don't overlap with LSP snippets
        result = ENV_VAR_BRACED.replace_all(&result, |caps: &regex::Captures| {
            let full_match = caps.get(0).unwrap();
            let match_start = full_match.start();
            
            if is_lsp_position(match_start) {
                // This is part of an LSP snippet, don't escape
                full_match.as_str().to_string()
            } else {
                // This is an environment variable, escape it
                format!("\\${{{}}}", &caps[1])
            }
        }).to_string();

        debug!("Escaped content: {:?}", result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_universal_snippet_when_translating_then_processes_content() {
        // Arrange
        let snippet = Snippet::new(
            1,
            "Test Universal Snippet".to_string(),
            "// This is a test".to_string(),
            "Test description".to_string(),
            vec!["universal".to_string(), "_snip_".to_string()],
        );
        let uri = Url::parse("file:///test.py").expect("parse URI");

        // Act
        let result = LanguageTranslator::translate_snippet(&snippet, "python", &uri);

        // Assert
        assert!(result.is_ok());
        let translated = result.expect("valid translation result");
        assert!(translated.contains("# This is a test"));
    }

    #[test]
    fn given_regular_snippet_when_translating_then_returns_as_is() {
        // Arrange
        let snippet = Snippet::new(
            1,
            "Test Regular Snippet".to_string(),
            "// This is a test".to_string(),
            "Test description".to_string(),
            vec!["rust".to_string(), "_snip_".to_string()],
        );
        let uri = Url::parse("file:///test.py").expect("parse URI");

        // Act
        let result = LanguageTranslator::translate_snippet(&snippet, "python", &uri);

        // Assert
        assert!(result.is_ok());
        let translated = result.expect("valid translation result");
        assert_eq!(translated, "// This is a test"); // No translation for non-universal
    }

    #[test]
    fn given_rust_line_comments_when_translating_to_python_then_converts_correctly() {
        // Arrange
        let uri = Url::parse("file:///test.py").expect("parse URI");
        let rust_content = r#"// This is a line comment
    // Indented comment
let x = 5; // End of line comment"#;

        // Act
        let result = LanguageTranslator::translate_rust_patterns(rust_content, "python", &uri);

        // Assert
        assert!(result.is_ok());
        let python_result = result.expect("Python translation result");
        assert!(python_result.contains("# This is a line comment"));
        assert!(python_result.contains("    # Indented comment"));
        assert!(python_result.contains("let x = 5; # End of line comment"));
    }

    #[test]
    fn given_rust_block_comments_when_translating_to_python_then_converts_correctly() {
        // Arrange
        let uri = Url::parse("file:///test.py").expect("parse URI");
        let rust_content = r#"/* This is a block comment */
/*
Multi-line
block comment
*/"#;

        // Act
        let result = LanguageTranslator::translate_rust_patterns(rust_content, "python", &uri);

        // Assert
        assert!(result.is_ok());
        let python_result = result.expect("Python translation result");
        assert!(python_result.contains("\"\"\" This is a block comment \"\"\""));
        assert!(python_result.contains("\"\"\"\nMulti-line\nblock comment\n\"\"\""));
    }

    #[test]
    fn given_rust_indentation_when_translating_to_go_then_converts_to_tabs() {
        // Arrange
        let uri = Url::parse("file:///test.go").expect("parse URI");
        let rust_content = r#"fn example() {
    let x = 5;
        let y = 10;
            let z = 15;
}"#;

        // Act
        let result = LanguageTranslator::translate_rust_patterns(rust_content, "go", &uri);

        // Assert
        assert!(result.is_ok());
        let go_result = result.expect("Go translation result");
        assert!(go_result.contains("fn example() {"));
        assert!(go_result.contains("\tlet x = 5;"));
        assert!(go_result.contains("\t\tlet y = 10;"));
        assert!(go_result.contains("\t\t\tlet z = 15;"));
    }

    #[test]
    fn given_filename_template_when_translating_then_replaces_correctly() {
        // Arrange
        let uri = Url::parse("file:///path/to/example.rs").expect("parse URI");
        let content = "// File: {{ filename }}";

        // Act
        let result = LanguageTranslator::translate_rust_patterns(content, "rust", &uri);

        // Assert
        assert!(result.is_ok());
        let translated = result.expect("valid translation result");
        assert!(translated.contains("// File: example.rs"));
    }

    #[test]
    fn given_environment_variables_when_escaping_enabled_then_escapes_env_vars() {
        // Arrange
        let content = "echo $HOME and ${USER} variables";

        // Act
        let result = LanguageTranslator::escape_environment_variables(content, true);

        // Assert
        assert_eq!(result, "echo \\$HOME and \\${USER} variables");
    }

    #[test]
    fn given_environment_variables_when_escaping_disabled_then_keeps_as_is() {
        // Arrange
        let content = "echo $HOME and ${USER} variables";

        // Act
        let result = LanguageTranslator::escape_environment_variables(content, false);

        // Assert
        assert_eq!(result, "echo $HOME and ${USER} variables");
    }

    #[test]
    fn given_lsp_snippet_syntax_when_escaping_then_preserves_lsp_syntax() {
        // Arrange
        let content = "function test($1, ${2:default}) { echo $HOME; $3 }";

        // Act
        let result = LanguageTranslator::escape_environment_variables(content, true);

        // Assert
        assert_eq!(result, "function test($1, ${2:default}) { echo \\$HOME; $3 }");
    }

    #[test]
    fn given_lsp_choice_syntax_when_escaping_then_preserves_choice_syntax() {
        // Arrange
        let content = "var type = ${1|string,number,boolean|}; echo $HOME";

        // Act
        let result = LanguageTranslator::escape_environment_variables(content, true);

        // Assert
        assert_eq!(result, "var type = ${1|string,number,boolean|}; echo \\$HOME");
    }

    #[test]
    fn given_mixed_dollar_signs_when_escaping_then_handles_correctly() {
        // Arrange
        let content = "Price: $100, Path: $HOME, Tabstop: $1, Placeholder: ${2:default}";

        // Act
        let result = LanguageTranslator::escape_environment_variables(content, true);

        // Assert
        assert_eq!(result, "Price: $100, Path: \\$HOME, Tabstop: $1, Placeholder: ${2:default}");
    }

    #[test]
    fn given_complex_snippet_when_escaping_then_handles_all_patterns() {
        // Arrange
        let content = r#"#!/bin/bash
# File: ${1:script.sh}
export PATH=$HOME/bin:$PATH
echo "User: $USER"
cd ${PROJECT_ROOT}
printf "${2|info,warn,error|}: %s\n" "$3""#;

        // Act
        let result = LanguageTranslator::escape_environment_variables(content, true);

        // Assert
        let expected = r#"#!/bin/bash
# File: ${1:script.sh}
export PATH=\$HOME/bin:\$PATH
echo "User: \$USER"
cd \${PROJECT_ROOT}
printf "${2|info,warn,error|}: %s\n" "$3""#;
        assert_eq!(result, expected);
    }

    #[test]
    fn given_edge_cases_when_escaping_then_handles_correctly() {
        // Arrange - test empty string and strings with no variables
        let empty = "";
        let no_vars = "just plain text with no variables";
        let just_dollar = "$";

        // Act
        let result_empty = LanguageTranslator::escape_environment_variables(empty, true);
        let result_no_vars = LanguageTranslator::escape_environment_variables(no_vars, true);
        let result_dollar = LanguageTranslator::escape_environment_variables(just_dollar, true);

        // Assert
        assert_eq!(result_empty, "");
        assert_eq!(result_no_vars, "just plain text with no variables");
        assert_eq!(result_dollar, "$"); // Just a dollar sign should not be escaped
    }
}
