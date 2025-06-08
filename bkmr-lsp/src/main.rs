use tower_lsp::{LspService, Server};
use bkmr_lsp::BkmrLspBackend;
use tracing_subscriber::{EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing to stderr only
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)  // Disable color codes
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("bkmr_lsp=info"))
        )
        .init();

    tracing::info!("Starting bkmr-lsp server");
    
    // Set up the LSP service
    let (service, socket) = LspService::new(|client| {
        tracing::debug!("Creating new LSP backend");
        BkmrLspBackend::new(client)
    });
    
    tracing::info!("LSP service created, starting server");
    
    // Start the server
    let server = Server::new(tokio::io::stdin(), tokio::io::stdout(), socket);
    
    server.serve(service).await;
    
    tracing::info!("Server shutting down");
    Ok(())
}