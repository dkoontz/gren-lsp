use crate::document_manager::{DocumentManager, DocumentManagerStats};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::{error, info};

pub struct GrenLspService {
    client: Client,
    document_manager: Arc<RwLock<DocumentManager>>,
}

impl GrenLspService {
    pub fn new(client: Client) -> Self {
        Self { 
            client,
            document_manager: Arc::new(RwLock::new(DocumentManager::new(100))), // LRU cache of 100 items
        }
    }

    /// Get document manager statistics for debugging
    pub async fn get_document_stats(&self) -> DocumentManagerStats {
        self.document_manager.read().await.get_stats()
    }

    /// Test-only method: Check if a document is currently open
    #[cfg(test)]
    pub async fn test_is_document_open(&self, uri: &Url) -> bool {
        self.document_manager.read().await.is_document_open(uri)
    }

    /// Test-only method: Check if a document is in the closed cache
    #[cfg(test)]
    pub async fn test_is_document_cached(&self, uri: &Url) -> bool {
        self.document_manager.write().await.is_document_cached(uri)
    }

    /// Test-only method: Get document content by URI
    #[cfg(test)]
    pub async fn test_get_document_content(&self, uri: &Url) -> Option<String> {
        self.document_manager.write().await.get_document_content(uri)
    }

    /// Test-only method: Get document version by URI
    #[cfg(test)]
    pub async fn test_get_document_version(&self, uri: &Url) -> Option<i32> {
        self.document_manager.write().await.get_document_version(uri)
    }

    /// Test-only method: Get cache capacity and usage info
    #[cfg(test)]
    pub async fn test_get_cache_info(&self) -> (usize, usize) {
        self.document_manager.read().await.get_cache_info()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for GrenLspService {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        info!("LSP initialize request received");
        
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "gren-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("LSP initialized notification received");
        
        self.client
            .log_message(MessageType::INFO, "Gren LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("LSP shutdown request received");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        info!("Document opened: {} (version {})", uri, version);
        
        let mut doc_manager = self.document_manager.write().await;
        if let Err(e) = doc_manager.did_open(params) {
            error!("Failed to open document {}: {}", uri, e);
            self.client
                .log_message(MessageType::ERROR, format!("Failed to open document: {}", e))
                .await;
        } else {
            let stats = doc_manager.get_stats();
            info!("Document manager stats: {} open, {} cached", stats.open_documents, stats.cached_documents);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        info!("Document changed: {} (version {})", uri, version);
        
        let mut doc_manager = self.document_manager.write().await;
        if let Err(e) = doc_manager.did_change(params) {
            error!("Failed to apply changes to document {}: {}", uri, e);
            self.client
                .log_message(MessageType::ERROR, format!("Failed to apply document changes: {}", e))
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        info!("Document saved: {}", params.text_document.uri);
        // For now, we don't need to do anything special on save
        // In the future, this might trigger diagnostics or other analysis
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        info!("Document closed: {}", uri);
        
        let mut doc_manager = self.document_manager.write().await;
        if let Err(e) = doc_manager.did_close(params) {
            error!("Failed to close document {}: {}", uri, e);
        } else {
            let stats = doc_manager.get_stats();
            info!("Document manager stats: {} open, {} cached", stats.open_documents, stats.cached_documents);
        }
    }

    // Placeholder implementations for language features
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        info!("Hover request at {:?}", params.text_document_position_params.position);
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        info!("Completion request at {:?}", params.text_document_position.position);
        Ok(None)
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        info!("Go to definition request at {:?}", params.text_document_position_params.position);
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        info!("References request at {:?}", params.text_document_position.position);
        Ok(None)
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        info!("Document symbol request for {:?}", params.text_document.uri);
        Ok(None)
    }

    async fn symbol(&self, params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>> {
        info!("Workspace symbol request for query: {:?}", params.query);
        Ok(None)
    }
}

#[cfg(test)]
impl GrenLspService {
    /// Test-only request handler for document state inspection
    pub async fn handle_test_request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match method {
            "test/isDocumentOpen" => {
                let uri_str = params.get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tower_lsp::jsonrpc::Error::invalid_params("Missing uri parameter"))?;
                let uri = Url::parse(uri_str)
                    .map_err(|_| tower_lsp::jsonrpc::Error::invalid_params("Invalid URI"))?;
                
                let is_open = self.test_is_document_open(&uri).await;
                Ok(serde_json::json!(is_open))
            },
            "test/isDocumentCached" => {
                let uri_str = params.get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tower_lsp::jsonrpc::Error::invalid_params("Missing uri parameter"))?;
                let uri = Url::parse(uri_str)
                    .map_err(|_| tower_lsp::jsonrpc::Error::invalid_params("Invalid URI"))?;
                
                let is_cached = self.test_is_document_cached(&uri).await;
                Ok(serde_json::json!(is_cached))
            },
            "test/getDocumentContent" => {
                let uri_str = params.get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tower_lsp::jsonrpc::Error::invalid_params("Missing uri parameter"))?;
                let uri = Url::parse(uri_str)
                    .map_err(|_| tower_lsp::jsonrpc::Error::invalid_params("Invalid URI"))?;
                
                let content = self.test_get_document_content(&uri).await;
                Ok(serde_json::json!(content))
            },
            "test/getDocumentVersion" => {
                let uri_str = params.get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tower_lsp::jsonrpc::Error::invalid_params("Missing uri parameter"))?;
                let uri = Url::parse(uri_str)
                    .map_err(|_| tower_lsp::jsonrpc::Error::invalid_params("Invalid URI"))?;
                
                let version = self.test_get_document_version(&uri).await;
                Ok(serde_json::json!(version))
            },
            "test/getCacheInfo" => {
                let (capacity, usage) = self.test_get_cache_info().await;
                Ok(serde_json::json!({"capacity": capacity, "usage": usage}))
            },
            _ => {
                Err(tower_lsp::jsonrpc::Error::method_not_found())
            }
        }
    }
}