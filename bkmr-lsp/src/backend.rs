use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::process::Command;
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
            document_cache: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Extract prefix from document at given position
    #[instrument(skip(self))]
    fn extract_prefix_at_position(&self, uri: &Url, position: Position) -> Option<String> {
        // For LSP servers, we typically need to track document content changes
        // For now, we'll implement a simple fallback that works with most editors
        
        // Try to get document content from cache
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
        
        // Extract word before cursor position
        let before_cursor = &line[..char_pos];
        
        // Find the start of the current word (alphanumeric + underscore)
        let word_start = before_cursor
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .map(|i| i + 1)
            .unwrap_or(0);
        
        let prefix = before_cursor[word_start..].trim();
        
        debug!("Extracted prefix: '{}' from line: '{}' at position: {}", prefix, line, char_pos);
        
        if prefix.is_empty() {
            None
        } else {
            Some(prefix.to_string())
        }
    }

    /// Execute bkmr command and return parsed snippets
    #[instrument(skip(self))]
    async fn fetch_snippets(&self, prefix: Option<&str>) -> Result<Vec<BkmrSnippet>> {
        let mut args = vec![
            "search".to_string(),
            "--json".to_string(),
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

        let output = Command::new(&self.config.bkmr_binary)
            .args(&args)
            .output()
            .map_err(|e| anyhow!("Failed to execute bkmr: {}", e))?;

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

        info!("Successfully fetched {} snippets", snippets.len());
        Ok(snippets)
    }

    /// Convert bkmr snippet to LSP completion item
    fn snippet_to_completion_item(
        &self,
        snippet: &BkmrSnippet,
        position: Position,
        prefix: Option<&str>,
    ) -> CompletionItem {
        let insert_text = snippet.url.clone(); // URL contains the actual snippet content
        let label = snippet.title.clone(); // Title is what user sees for selection

        // Calculate the range for text replacement based on prefix
        let replace_range = if let Some(prefix_str) = prefix {
            if !prefix_str.is_empty() && position.character >= prefix_str.len() as u32 {
                Range {
                    start: Position {
                        line: position.line,
                        character: position.character - prefix_str.len() as u32,
                    },
                    end: position,
                }
            } else {
                Range {
                    start: position,
                    end: position,
                }
            }
        } else {
            Range {
                start: position,
                end: position,
            }
        };

        CompletionItem {
            label: label.clone(),
            kind: Some(CompletionItemKind::SNIPPET),
            // detail: Some(format!("Tags: {}", snippet.tags.join(", "))),
            // not working, only shows snippet
            // label_details: Some(CompletionItemLabelDetails {
            //     detail: Some("bkmr".to_string()),
            //     description: Some("bkmr".to_string()),
            // }),
            documentation: Some(Documentation::String(format!(
                "{}",
                if insert_text.len() > 700 {
                    format!("{}...", &insert_text[..700])
                } else {
                    insert_text.clone()
                }
            ))),
            insert_text: Some(insert_text.clone()),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            // Use filter_text to control what text is used for filtering
            filter_text: Some(label.clone()),
            // Use sort_text to control ordering - alphabetical by title
            sort_text: Some(label.clone()),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: replace_range,
                new_text: insert_text,
            })),
            data: Some(serde_json::json!({
                "id": snippet.id,
                "type": "bkmr_snippet"
            })),
            ..Default::default()
        }
    }

    /// Check if bkmr binary is available
    #[instrument(skip(self))]
    async fn verify_bkmr_availability(&self) -> Result<()> {
        debug!("Verifying bkmr availability");

        let output = Command::new(&self.config.bkmr_binary)
            .args(&["--help"])
            .output()
            .map_err(|e| anyhow!("bkmr binary not found: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!("bkmr binary is not working properly"));
        }

        info!("bkmr binary verified successfully");
        Ok(())
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
                    TextDocumentSyncKind::FULL
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    completion_item: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["bkmr.open".to_string(), "bkmr.refresh".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        info!("Initialize complete, capabilities: completion and execute_command");
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
        
        debug!("Completion request for {}:{},{}", uri, position.line, position.character);
        
        // Extract prefix from current position
        let prefix = self.extract_prefix_at_position(uri, position);
        
        debug!("Extracted prefix: {:?}", prefix);
        
        match self.fetch_snippets(prefix.as_deref()).await {
            Ok(snippets) => {
                let completion_items: Vec<CompletionItem> = snippets
                    .iter()
                    .map(|snippet| self.snippet_to_completion_item(snippet, position, prefix.as_deref()))
                    .collect();

                info!("Returning {} completion items for prefix: {:?}", completion_items.len(), prefix);

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
        info!("Execute command: {}", params.command);

        match params.command.as_str() {
            "bkmr.refresh" => {
                // This is now a no-op since we don't cache, but kept for compatibility
                self.client
                    .show_message(
                        MessageType::INFO,
                        "No cache to refresh - snippets are fetched live",
                    )
                    .await;
                Ok(None)
            }
            "bkmr.open" => {
                if !params.arguments.is_empty() {
                    if let Some(id_value) = params.arguments.get(0) {
                        if let Some(id) = id_value.as_i64() {
                            match Command::new(&self.config.bkmr_binary)
                                .args(&["open", &id.to_string()])
                                .status()
                            {
                                Ok(_) => {
                                    info!("Successfully opened bookmark {}", id);
                                    self.client
                                        .show_message(
                                            MessageType::INFO,
                                            &format!("Opened bookmark {}", id),
                                        )
                                        .await;
                                }
                                Err(e) => {
                                    error!("Failed to open bookmark {}: {}", id, e);
                                    self.client
                                        .show_message(
                                            MessageType::ERROR,
                                            &format!("Failed to open bookmark {}: {}", id, e),
                                        )
                                        .await;
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            _ => {
                warn!("Unknown command: {}", params.command);
                self.client
                    .log_message(
                        MessageType::WARNING,
                        &format!("Unknown command: {}", params.command),
                    )
                    .await;
                Ok(None)
            }
        }
    }
}
