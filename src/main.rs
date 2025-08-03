use anyhow::Result;
use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};
use tracing::info;

mod lsp_service;
mod document_manager;
mod compiler_interface;
mod diagnostics;
mod symbol_index;
mod tree_sitter_queries;
mod gren_language;
mod completion;
mod scope_analysis;

use lsp_service::GrenLspService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing - write to stderr to avoid interfering with LSP protocol on stdout
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
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