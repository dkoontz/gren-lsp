use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tower_lsp::{LspService, Server};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod server;
use server::GrenLanguageServer;

mod test_utils;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug mode to export tree-sitter parse trees
    #[arg(long, help = "Export tree-sitter parse trees to specified directory for debugging")]
    debug_export_trees: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging with file output for debugging
    let log_dir = std::env::temp_dir().join("gren-lsp");
    std::fs::create_dir_all(&log_dir).ok();
    let log_file = log_dir.join("server.log");

    let file_appender = tracing_appender::rolling::never(&log_dir, "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("gren_lsp=debug")),
        )
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    info!("LSP server starting up");
    info!("Log file location: {}", log_file.display());

    info!("Starting Gren Language Server");

    // Create the transport for stdio
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Log debug settings
    if let Some(ref debug_dir) = args.debug_export_trees {
        info!("Parse tree debug export enabled, directory: {}", debug_dir.display());
    }

    // Create the language server
    info!("Creating language server service");
    let (service, socket) = LspService::new(move |client| {
        info!("Creating new language server instance");
        GrenLanguageServer::new_with_debug(client, args.debug_export_trees.clone())
    });

    // Run the server
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
