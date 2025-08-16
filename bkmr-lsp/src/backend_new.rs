use anyhow::{Context, Result};
use std::sync::Arc;
use tower_lsp::{
    jsonrpc::Result as LspResult, Client, LanguageServer,
    lsp_types::*,
};
use tracing::{debug, error, info, instrument, warn};

use crate::repositories::{BkmrRepository, RepositoryConfig, SnippetRepository};
use crate::services::{CommandService, CompletionService, DocumentService};

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

impl From<BkmrConfig> for RepositoryConfig {
    fn from(config: BkmrConfig) -> Self {
        Self {
            binary_path: config.bkmr_binary,
            max_results: config.max_completions,
            timeout_seconds: 10,
        }
    }
}

/// Refactored LSP backend using clean architecture principles
#[derive(Debug)]
pub struct BkmrLspBackend {
    client: Client,
    config: BkmrConfig,
    document_service: Arc<DocumentService>,
    completion_service: Arc<CompletionService>,
}

impl BkmrLspBackend {
    /// Create a new backend with dependency injection
    pub fn new(client: Client) -> Self {
        let config = BkmrConfig::default();
        Self::with_config(client, config)
    }

    /// Create a new backend with custom configuration
    pub fn with_config(client: Client, config: BkmrConfig) -> Self {
        debug!("Creating BkmrLspBackend with config: {:?}", config);

        let repository_config = RepositoryConfig::from(config.clone());
        let repository: Arc<dyn SnippetRepository> = Arc::new(BkmrRepository::new(repository_config));
        
        let document_service = Arc::new(DocumentService::new());
        let completion_service = Arc::new(CompletionService::new(repository));

        Self {
            client,
            config,
            document_service,
            completion_service,
        }
    }

    /// Create a new backend with custom repository (for testing)
    #[cfg(test)]
    pub fn with_repository(
        client: Client, 
        config: BkmrConfig, 
        repository: Arc<dyn SnippetRepository>
    ) -> Self {
        debug!("Creating BkmrLspBackend with custom repository");

        let document_service = Arc::new(DocumentService::new());
        let completion_service = Arc::new(CompletionService::new(repository));

        Self {
            client,
            config,
            document_service,
            completion_service,
        }
    }

    /// Check if repository is available
    #[instrument(skip(self))]
    async fn verify_repository_availability(&self) -> Result<()> {
        self.completion_service
            .health_check()
            .await
            .context("verify repository availability")
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

        // Verify repository is available
        if let Err(e) = self.verify_repository_availability().await {
            error!("Repository verification failed: {}", e);
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

        if let Err(e) = self.document_service
            .open_document(uri, language_id, content)
            .await
        {
            error!("Failed to open document: {}", e);
        }
    }

    #[instrument(skip(self, params))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        debug!("Document changed: {}", uri);

        for change in params.content_changes {
            // For FULL sync, replace entire content
            if change.range.is_none() {
                if let Err(e) = self.document_service
                    .update_document(uri.clone(), change.text)
                    .await
                {
                    error!("Failed to update document: {}", e);
                }
            } else {
                // For incremental sync, would need more complex logic
                // For now, just replace entirely
                if let Err(e) = self.document_service
                    .update_document(uri.clone(), change.text)
                    .await
                {
                    error!("Failed to update document: {}", e);
                }
            }
        }
    }

    #[instrument(skip(self, params))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        debug!("Document closed: {}", uri);

        if let Err(e) = self.document_service.close_document(uri).await {
            error!("Failed to close document: {}", e);
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

        // Extract completion context
        let completion_context = match self.document_service
            .extract_completion_context(uri, position)
            .await
        {
            Ok(context) => context,
            Err(e) => {
                error!("Failed to extract completion context: {}", e);
                return Ok(Some(CompletionResponse::Array(vec![])));
            }
        };

        debug!("Completion context: language_id={:?}, has_query={}", 
               completion_context.language_id, completion_context.has_query());

        match self.completion_service.get_completions(&completion_context).await {
            Ok(completion_items) => {
                info!(
                    "Returning {} completion items for query: {:?}",
                    completion_items.len(),
                    completion_context.get_query_text()
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
                error!("Failed to get completions: {}", e);
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
                            match CommandService::insert_filepath_comment(&uri_str) {
                                Ok(workspace_edit) => {
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

/// Start a bkmr-lsp server with given input/output streams
/// This function is used by tests to spawn a real LSP server for testing
pub async fn start_server<I, O>(read: I, write: O) 
where
    I: tokio::io::AsyncRead + Unpin,
    O: tokio::io::AsyncWrite,
{
    use tower_lsp::{LspService, Server};
    
    // Create the LSP service
    let (service, socket) = LspService::new(|client| {
        BkmrLspBackend::new(client)
    });
    
    // Start the server with the provided streams
    Server::new(read, write, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::MockSnippetRepository;
    use crate::domain::Snippet;

    fn create_mock_client() -> Client {
        let (service, _socket) = tower_lsp::LspService::new(|client| {
            BkmrLspBackend::new(client)
        });
        // This is a bit of a hack for testing, but we need a client instance
        // In real usage, the client is provided by the LSP framework
        service.inner().client.clone()
    }

    #[tokio::test]
    async fn given_new_backend_when_created_then_initializes_correctly() {
        // Arrange
        let client = create_mock_client();

        // Act
        let backend = BkmrLspBackend::new(client);

        // Assert
        assert_eq!(backend.config.bkmr_binary, "bkmr");
        assert_eq!(backend.config.max_completions, 50);
    }

    #[tokio::test]
    async fn given_backend_with_mock_repository_when_completing_then_uses_repository() {
        // Arrange
        let snippet = Snippet::new(
            1,
            "Test Snippet".to_string(),
            "test content".to_string(),
            "Test description".to_string(),
            vec!["rust".to_string(), "_snip_".to_string()],
        );

        let repository = Arc::new(
            MockSnippetRepository::new()
                .with_snippets(vec![snippet])
        );

        let client = create_mock_client();
        let config = BkmrConfig::default();
        let backend = BkmrLspBackend::with_repository(client, config, repository);

        // First, simulate opening a document
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse("file:///test.rs").expect("parse URI"),
                language_id: "rust".to_string(),
                version: 1,
                text: "test".to_string(),
            },
        };
        backend.did_open(did_open_params).await;

        // Act
        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse("file:///test.rs").expect("parse URI"),
                },
                position: Position { line: 0, character: 4 },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        };

        let result = backend.completion(completion_params).await;

        // Assert
        assert!(result.is_ok());
        let response = result.expect("completion LSP result").expect("completion response");
        
        match response {
            CompletionResponse::List(list) => {
                assert_eq!(list.items.len(), 1);
                assert_eq!(list.items[0].label, "Test Snippet");
            }
            _ => panic!("Expected completion list"),
        }
    }
}