use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
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
    config: Arc<RwLock<BkmrConfig>>,
    cached_snippets: Arc<RwLock<Vec<BkmrSnippet>>>,
}

impl BkmrLspBackend {
    pub fn new(client: Client) -> Self {
        debug!("Creating BkmrLspBackend");
        Self {
            client,
            config: Arc::new(RwLock::new(BkmrConfig::default())),
            cached_snippets: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Execute bkmr command and return parsed snippets with timeout
    #[instrument(skip(self))]
    async fn fetch_snippets(&self, prefix: Option<&str>) -> Result<Vec<BkmrSnippet>> {
        let config = self.config.read().await;
        
        let mut args = vec![
            "search".to_string(),
            "--json".to_string(),
            "-t".to_string(),
            "_snip_".to_string(),
            "--limit".to_string(),
            config.max_completions.to_string(),
        ];

        // Add search term if prefix is provided and not empty
        if let Some(p) = prefix {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                args.push(format!("metadata:{}", trimmed));
                debug!("Using search prefix: {}", trimmed);
            }
        }

        debug!("Executing bkmr with args: {:?}", args);

        // Add timeout to prevent hanging
        let command_future = tokio::process::Command::new(&config.bkmr_binary)
            .args(&args)
            .output();

        let output = match tokio::time::timeout(std::time::Duration::from_secs(10), command_future).await {
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
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!("bkmr command failed. Exit code: {:?}", output.status.code());
            error!("stderr: {}", stderr);
            error!("stdout: {}", stdout);
            return Err(anyhow!("bkmr command failed with exit code: {:?}", output.status.code()));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let trimmed_output = stdout_str.trim();

        if trimmed_output.is_empty() {
            debug!("Empty output from bkmr");
            return Ok(Vec::new());
        }

        // Handle case where output might not be valid JSON array
        let snippets: Vec<BkmrSnippet> = match serde_json::from_str(trimmed_output) {
            Ok(snippets) => snippets,
            Err(e) => {
                error!("Failed to parse bkmr JSON output: {}", e);
                error!("Raw output was: {}", trimmed_output);
                
                // Try to parse as single object and wrap in array
                match serde_json::from_str::<BkmrSnippet>(trimmed_output) {
                    Ok(single_snippet) => {
                        debug!("Parsed single snippet, wrapping in array");
                        vec![single_snippet]
                    }
                    Err(_) => {
                        return Err(anyhow!("Failed to parse bkmr JSON output: {}", e));
                    }
                }
            }
        };

        info!("Successfully fetched {} snippets", snippets.len());
        
        // Update cache
        let mut cache = self.cached_snippets.write().await;
        *cache = snippets.clone();
        
        Ok(snippets)
    }

    /// Convert bkmr snippet to LSP completion item
    fn snippet_to_completion_item(&self, snippet: &BkmrSnippet) -> CompletionItem {
        let insert_text = snippet.url.clone(); // URL contains the actual snippet content
        let label = snippet.title.clone(); // Title is what user sees for selection

        // Create documentation with snippet preview
        let doc_text = if snippet.description.is_empty() {
            format!(
                "ID: {}\nTags: {}\n\n{}",
                snippet.id,
                snippet.tags.join(", "),
                if insert_text.len() > 300 {
                    format!("{}...", &insert_text[..300])
                } else {
                    insert_text.clone()
                }
            )
        } else {
            format!(
                "ID: {}\nDescription: {}\nTags: {}\n\n{}",
                snippet.id,
                snippet.description,
                snippet.tags.join(", "),
                if insert_text.len() > 200 {
                    format!("{}...", &insert_text[..200])
                } else {
                    insert_text.clone()
                }
            )
        };

        CompletionItem {
            label: label.clone(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(format!("bkmr snippet #{}", snippet.id)),
            documentation: Some(Documentation::String(doc_text)),
            insert_text: Some(insert_text.clone()),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            filter_text: Some(label.clone()),
            // Use sort_text to control ordering - alphabetical by title
            sort_text: Some(label.clone()),
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
        let config = self.config.read().await;
        debug!("Verifying bkmr availability for binary: {}", config.bkmr_binary);

        let command_future = tokio::process::Command::new(&config.bkmr_binary)
            .args(&["--version"])
            .output();

        let output = match tokio::time::timeout(std::time::Duration::from_secs(5), command_future).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(anyhow!("bkmr binary not found or not executable: {}", e));
            }
            Err(_) => {
                return Err(anyhow!("bkmr --version command timed out"));
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bkmr binary check failed: {}", stderr));
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        info!("bkmr binary verified successfully: {}", version_output.trim());
        Ok(())
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BkmrLspBackend {
    #[instrument(skip(self, params))]
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        info!("Initialize request received");
        
        if let Some(client_info) = &params.client_info {
            info!("Client: {} {}", client_info.name, client_info.version.as_deref().unwrap_or("unknown"));
        }

        // Check configuration from initialization options
        if let Some(init_opts) = params.initialization_options {
            if let Ok(max_completions) = init_opts.get("maxCompletions")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .ok_or(())
            {
                let mut config = self.config.write().await;
                config.max_completions = max_completions;
                info!("Set max_completions to {}", max_completions);
            }
        }

        // Verify bkmr is available but don't fail initialization
        match self.verify_bkmr_availability().await {
            Ok(_) => {
                info!("bkmr binary verification successful");
                
                // Pre-load snippets in background
                let backend = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = backend.fetch_snippets(None).await {
                        warn!("Failed to pre-load snippets: {}", e);
                    } else {
                        info!("Successfully pre-loaded snippets");
                    }
                });
            }
            Err(e) => {
                warn!("bkmr verification failed (server will continue): {}", e);
            }
        }

        let result = InitializeResult {
            capabilities: ServerCapabilities {
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    // trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    completion_item: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["bkmr.open".to_string(), "bkmr.refresh".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::NONE
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "bkmr-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        };

        info!("Initialize complete");
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
    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        
        debug!("Completion request for {}:{},{}", uri, position.line, position.character);
        
        // Try to get snippets, but always return something
        let snippets = match self.fetch_snippets(None).await {
            Ok(snippets) => {
                debug!("Successfully fetched {} snippets", snippets.len());
                snippets
            }
            Err(e) => {
                warn!("Failed to fetch snippets: {}, trying cache", e);
                
                // Try to use cached snippets
                let cache = self.cached_snippets.read().await;
                if cache.is_empty() {
                    warn!("No cached snippets available");
                    return Ok(Some(CompletionResponse::Array(vec![])));
                } else {
                    debug!("Using {} cached snippets", cache.len());
                    cache.clone()
                }
            }
        };

        if snippets.is_empty() {
            debug!("No snippets available");
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        let completion_items: Vec<CompletionItem> = snippets
            .iter()
            .map(|snippet| self.snippet_to_completion_item(snippet))
            .collect();

        info!("Returning {} completion items", completion_items.len());
        Ok(Some(CompletionResponse::Array(completion_items)))
    }

    #[instrument(skip(self, params))]
    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> LspResult<Option<serde_json::Value>> {
        info!("Execute command: {}", params.command);

        let config = self.config.read().await;
        
        match params.command.as_str() {
            "bkmr.refresh" => {
                info!("Refreshing snippets cache");
                match self.fetch_snippets(None).await {
                    Ok(snippets) => {
                        info!("Successfully refreshed {} snippets", snippets.len());
                    }
                    Err(e) => {
                        warn!("Failed to refresh snippets: {}", e);
                    }
                }
                Ok(None)
            }
            "bkmr.open" => {
                if let Some(id_value) = params.arguments.get(0) {
                    if let Some(id) = id_value.as_i64() {
                        debug!("Opening bookmark {}", id);
                        
                        let command_future = tokio::process::Command::new(&config.bkmr_binary)
                            .args(&["open", &id.to_string()])
                            .status();

                        match tokio::time::timeout(std::time::Duration::from_secs(10), command_future).await {
                            Ok(Ok(status)) => {
                                if status.success() {
                                    info!("Successfully opened bookmark {}", id);
                                } else {
                                    warn!("bkmr open command failed for bookmark {} with exit code: {:?}", id, status.code());
                                }
                            }
                            Ok(Err(e)) => {
                                error!("Failed to execute bkmr open for bookmark {}: {}", id, e);
                            }
                            Err(_) => {
                                error!("bkmr open command timed out for bookmark {}", id);
                            }
                        }
                    } else {
                        warn!("Invalid bookmark ID in bkmr.open command: {:?}", id_value);
                    }
                } else {
                    warn!("No bookmark ID provided for bkmr.open command");
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

// Implement Clone for background task spawning
impl Clone for BkmrLspBackend {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: Arc::clone(&self.config),
            cached_snippets: Arc::clone(&self.cached_snippets),
        }
    }
}