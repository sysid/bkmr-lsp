use bkmr_lsp::backend::{BkmrLspBackend, BkmrConfig};
use clap::Parser;
use std::panic;
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "bkmr-lsp")]
#[command(about = "Language Server Protocol implementation for bkmr snippet manager")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Disable environment variable escaping in LSP snippets
    #[arg(long, help = "Disable escaping of environment variables ($VAR) in snippet content")]
    no_escape_vars: bool,
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();
    // Set up panic hook to log panics instead of just exiting
    panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC in bkmr-lsp: {}", panic_info);

        // Try to log to tracing if available
        if let Some(location) = panic_info.location() {
            eprintln!(
                "Panic occurred in file '{}' at line {}",
                location.file(),
                location.line()
            );
        }

        // Print payload if available
        if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("Panic payload: {}", payload);
        } else if let Some(payload) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("Panic payload: {}", payload);
        }

        std::process::exit(1);
    }));

    // Initialize logging with fallback if it fails
    let result = init_logging();
    if let Err(e) = result {
        eprintln!(
            "Failed to initialize logging: {}, continuing without structured logging",
            e
        );
    }

    tracing::info!("Starting bkmr-lsp server v{}", env!("CARGO_PKG_VERSION"));

    // Create configuration from CLI args
    let config = BkmrConfig {
        escape_variables: !args.no_escape_vars,
        ..Default::default()
    };

    tracing::info!("Configuration: {:?}", config);

    // Validate environment before starting
    if let Err(e) = validate_environment().await {
        tracing::error!("Environment validation failed: {}", e);
        eprintln!("Environment validation failed: {}", e);
        std::process::exit(1);
    }

    // Set up the LSP service with error handling
    let (service, socket) = LspService::new(move |client| {
        tracing::debug!("Creating new LSP backend instance");
        BkmrLspBackend::with_config(client, config.clone())
    });

    tracing::info!("LSP service created, starting server on stdin/stdout");

    // Create server with stdin/stdout
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Start the server - this method returns () and only exits on error via panic
    tracing::info!("Starting LSP server loop");
    Server::new(stdin, stdout, socket).serve(service).await;

    // If we reach here, the server has shut down gracefully
    tracing::info!("Server shutdown gracefully");
}

/// Initialize logging with fallback options
fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Try different logging configurations in order of preference
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("bkmr_lsp=info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false) // Disable color codes for LSP compatibility
        .with_target(false) // Reduce noise in LSP logs
        .with_env_filter(filter)
        .try_init()?;

    Ok(())
}

/// Validate that the environment is suitable for running the LSP server
async fn validate_environment() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check if we're in a proper LSP context (stdin/stdout should be available)
    if atty::is(atty::Stream::Stdin) || atty::is(atty::Stream::Stdout) {
        eprintln!("Warning: bkmr-lsp is designed to run as an LSP server");
        eprintln!("It should be launched by an LSP client, not directly from a terminal");
        eprintln!("If you're testing, pipe some LSP messages to stdin");
    }

    // Test basic async functionality
    tokio::time::timeout(std::time::Duration::from_millis(100), async {
        tokio::task::yield_now().await
    })
    .await?;

    Ok(())
}
