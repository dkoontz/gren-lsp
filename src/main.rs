use anyhow::Result;
use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};
use tracing::info;

mod lsp_service;

use lsp_service::GrenLspService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Gren LSP Server");

    // Create LSP service
    let (service, socket) = LspService::new(|client| GrenLspService::new(client));
    
    // Run the server with stdio transport
    let server = Server::new(stdin(), stdout(), socket);
    server.serve(service).await;

    info!("Gren LSP Server stopped");
    Ok(())
}