use gren_lsp_core::workspace::Workspace;
use gren_lsp_protocol::handlers::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer};
use tracing::{error, info};

pub struct GrenLanguageServer {
    client: Client,
    workspace: Arc<RwLock<Workspace>>,
}

impl GrenLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspace: Arc::new(RwLock::new(Workspace::new())),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for GrenLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Initialize request received");

        // Set workspace root
        if let Some(root_uri) = params.root_uri {
            let mut workspace = self.workspace.write().await;
            workspace.set_root(root_uri);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("Server initialized");
        
        // TODO: Start indexing workspace
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutdown request received");
        Ok(())
    }

    // Document synchronization
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        info!("Document opened: {}", params.text_document.uri);
        
        let mut workspace = self.workspace.write().await;
        workspace.open_document(params.text_document);
        
        // TODO: Trigger initial analysis
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut workspace = self.workspace.write().await;
        
        if let Err(e) = workspace.update_document(params) {
            error!("Failed to update document: {}", e);
        }
        
        // TODO: Trigger incremental analysis
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        info!("Document closed: {}", params.text_document.uri);
        
        let mut workspace = self.workspace.write().await;
        workspace.close_document(params.text_document.uri);
    }

    // Language features
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let handlers = Handlers::new(self.workspace.clone());
        handlers.hover(params).await
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let handlers = Handlers::new(self.workspace.clone());
        handlers.completion(params).await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let handlers = Handlers::new(self.workspace.clone());
        handlers.goto_definition(params).await
    }
}