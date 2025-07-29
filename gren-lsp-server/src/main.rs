use anyhow::Result;
use tower_lsp::{LspService, Server};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod server;
use server::GrenLanguageServer;

mod test_utils;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("gren_lsp=info")),
        )
        .init();

    info!("Starting Gren Language Server");

    // Create the transport for stdio
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Create the language server
    let (service, socket) = LspService::new(|client| GrenLanguageServer::new(client));

    // Run the server
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}