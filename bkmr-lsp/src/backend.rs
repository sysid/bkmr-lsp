// File: bkmr-lsp/src/backend.rs - Updated completion logic with trigger characters

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tower_lsp::{Client, LanguageServer, jsonrpc::Result as LspResult, lsp_types::*};
use tracing::{debug, error, info, instrument, warn};

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
        }
    }

    /// Extract prefix from document at given position
    #[instrument(skip(self))]
    fn extract_snippet_query(&self, uri: &Url, position: Position) -> Option<String> {
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

        // Find the last ':' and check if it's a valid snippet trigger
        if let Some(trigger_pos) = before_cursor.rfind(':') {
            let after_trigger = &before_cursor[trigger_pos + 1..];

            // Check if this might be part of a URL, time, or other non-snippet context
            if trigger_pos > 0 {
                let char_before = before_cursor.chars().nth(trigger_pos - 1);
                if let Some(prev_char) = char_before {
                    // Skip if it looks like URL (http:), time (12:30), or path (C:\)
                    if prev_char.is_alphanumeric() || prev_char == 'p' || prev_char == 't' {
                        return None;
                    }
                }
            }

            // Only proceed if:
            // 1. No whitespace immediately after ':'
            // 2. All characters are valid identifier chars
            // 3. We either have content or just typed ':'
            if !after_trigger.starts_with(char::is_whitespace)
                && after_trigger
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Some(after_trigger.to_string());
            }
        }

        None
    }

    /// Execute bkmr command and return parsed snippets
    #[instrument(skip(self))]
    async fn fetch_snippets(&self, prefix: Option<&str>) -> Result<Vec<BkmrSnippet>> {
        let mut args = vec![
            "search".to_string(),
            "--json".to_string(),
            "--interpolate".to_string(), // ← ADD THIS: Always use interpolation
            "-t".to_string(),
            "_snip_".to_string(),
            "--limit".to_string(),
            self.config.max_completions.to_string(),
        ];

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

    /// Convert bkmr snippet to LSP completion item with trigger-aware text replacement
    fn snippet_to_completion_item_with_trigger(
        &self,
        snippet: &BkmrSnippet,
        _position: Position,
        query: &str,
    ) -> CompletionItem {
        let original_insert_text = snippet.url.clone();
        let label = snippet.title.clone();

        // Check if insertText starts with query (case insensitive)
        let insert_matches = original_insert_text
            .to_lowercase()
            .starts_with(&query.to_lowercase());

        // If insertText doesn't match query, prefix it with the query (vim lsp workaround)
        let insert_text = if !insert_matches && !query.is_empty() {
            format!("{} {}", query, original_insert_text)
        } else {
            original_insert_text
        };

        debug!(
            "Completion item: query='{}', label='{}', insertText='{}' (fixed={})",
            query,
            label,
            insert_text.chars().take(20).collect::<String>(),
            !insert_matches
        );

        CompletionItem {
            label: label.clone(),
            kind: Some(CompletionItemKind::TEXT),
            // detail: Some(format!("bkmr #{}", snippet.id)),
            documentation: Some(Documentation::String(if insert_text.len() > 500 {
                format!("{}...", &insert_text[..500])
            } else {
                insert_text.clone()
            })),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            ..Default::default()
        }
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

    /// Determine the appropriate comment syntax for a file based on its extension
    fn get_comment_syntax(&self, file_path: &str) -> &'static str {
        let path = Path::new(file_path);
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match extension {
            // C-style languages
            "rs" | "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "java" | "js" | "ts" | "jsx"
            | "tsx" | "cs" | "go" | "swift" | "kt" | "scala" | "dart" => "//",
            // Shell-style languages
            "sh" | "bash" | "zsh" | "fish" | "py" | "rb" | "pl" | "r" | "yaml" | "yml" | "toml"
            | "cfg" | "ini" | "properties" => "#",
            // HTML/XML
            "html" | "htm" | "xml" | "xhtml" | "svg" => "<!--",
            // CSS
            "css" | "scss" | "sass" | "less" => "/*",
            // SQL
            "sql" => "--",
            // Lua
            "lua" => "--",
            // Haskell
            "hs" => "--",
            // Lisp family
            "lisp" | "cl" | "clj" | "cljs" | "scm" | "rkt" => ";",
            // VimScript
            "vim" => "\"",
            // Batch files
            "bat" | "cmd" => "REM",
            // PowerShell
            "ps1" | "psm1" | "psd1" => "#",
            // LaTeX
            "tex" | "latex" => "%",
            // Fortran
            "f" | "f77" | "f90" | "f95" | "f03" | "f08" => "!",
            // MATLAB
            "m" => "%",
            // Default to hash for unknown file types
            _ => "#",
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
                    trigger_characters: Some(vec![":".to_string()]),
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

        info!("Initialize complete - trigger characters: [':']");
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

        debug!("Document opened: {}", uri);

        if let Ok(mut cache) = self.document_cache.write() {
            cache.insert(uri, content);
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
    }

    #[instrument(skip(self, params))]
    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        debug!(
            "Completion request for {}:{},{}",
            uri, position.line, position.character
        );

        // Check if this was triggered by our trigger character
        if let Some(context) = &params.context {
            match context.trigger_kind {
                CompletionTriggerKind::TRIGGER_CHARACTER => {
                    // This is exactly what we want - triggered by typing ':'
                    if let Some(trigger_char) = &context.trigger_character {
                        if trigger_char == ":" {
                            debug!(
                                "Triggered by ':' character - proceeding with snippet completion"
                            );
                        } else {
                            debug!(
                                "Triggered by different character '{}' - skipping",
                                trigger_char
                            );
                            return Ok(Some(CompletionResponse::Array(vec![])));
                        }
                    }
                }
                CompletionTriggerKind::INVOKED => {
                    // Manual Ctrl+Space - check if we're in a snippet context
                    let query = self.extract_snippet_query(uri, position);
                    if query.is_none() {
                        debug!("Manual trigger but no snippet context - skipping");
                        return Ok(Some(CompletionResponse::Array(vec![])));
                    }
                    debug!("Manual trigger with snippet context - proceeding");
                }
                CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS => {
                    debug!("Completion for incomplete results - proceeding");
                }
                _ => {
                    debug!("Unknown trigger kind - skipping");
                    return Ok(Some(CompletionResponse::Array(vec![])));
                }
            }
        } else {
            debug!("No completion context - skipping");
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        // Extract the query after trigger
        let query = self.extract_snippet_query(uri, position);
        debug!("Extracted snippet query: {:?}", query);

        // Determine what text to replace (for proper text editing)
        // Extract data from cache and drop the guard before await
        let trigger_context = {
            let cache = self.document_cache.read().unwrap();
            let content = cache.get(&uri.to_string()).map(|s| &**s).unwrap_or("");
            let lines: Vec<&str> = content.lines().collect();
            let line = if (position.line as usize) < lines.len() {
                lines[position.line as usize]
            } else {
                ""
            };
            let before_cursor = &line[..std::cmp::min(position.character as usize, line.len())];

            // Find the trigger context for text replacement
            if let Some(pos) = before_cursor.rfind(":snip:") {
                before_cursor[pos..].to_string()
            } else if let Some(pos) = before_cursor.rfind(":s:") {
                before_cursor[pos..].to_string()
            } else if let Some(pos) = before_cursor.rfind(':') {
                before_cursor[pos..].to_string()
            } else {
                String::new()
            }
        }; // Guard is dropped here

        debug!("Trigger context: '{}'", trigger_context);

        match self.fetch_snippets(query.as_deref()).await {
            Ok(snippets) => {
                let query_str = query.as_deref().unwrap_or(""); // Clean query: "aws", not ":aws"
                let completion_items: Vec<CompletionItem> = snippets
                    .iter()
                    .map(|snippet| {
                        self.snippet_to_completion_item_with_trigger(
                            snippet, position, query_str, // Pass "aws", not ":aws"
                        )
                    })
                    .collect();

                info!(
                    "Returning {} completion items for query: {:?}",
                    completion_items.len(),
                    query
                );

                for (i, item) in completion_items.iter().enumerate() {
                    debug!(
                        "Item {}: label='{}', sort_text={:?}",
                        i, item.label, item.sort_text
                    );
                }

                Ok(Some(CompletionResponse::Array(completion_items)))
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
