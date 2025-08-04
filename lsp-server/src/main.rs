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
mod hover;
mod goto_definition;
mod find_references;
mod document_symbols;
mod code_actions;
mod workspace_symbols;
mod rename;
mod performance;
mod module_rename;
mod file_operations;
mod import_rewriter;
mod import_manager;
mod import_completion;
mod workspace_protocol;

use lsp_service::GrenLspService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing - write to stderr to avoid interfering with LSP protocol on stdout
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Gren LSP Server");

    // Create LSP service with custom test methods
    let (service, socket) = LspService::build(|client| GrenLspService::new(client))
        .custom_method("test/getDocumentVersion", GrenLspService::handle_get_document_version_custom)
        .custom_method("test/getDocumentContent", GrenLspService::handle_get_document_content_custom)
        .custom_method("test/isDocumentOpen", GrenLspService::handle_is_document_open_custom)
        .custom_method("test/isDocumentCached", GrenLspService::handle_is_document_cached_custom)
        .custom_method("test/getCacheInfo", GrenLspService::handle_get_cache_info_custom)
        .finish();
    
    // Run the server with stdio transport
    let server = Server::new(stdin(), stdout(), socket);
    server.serve(service).await;

    info!("Gren LSP Server stopped");
    Ok(())
}