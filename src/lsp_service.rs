use crate::document_manager::{DocumentManager, DocumentManagerStats};
use crate::compiler_interface::{GrenCompiler, CompileRequest, CompilerConfig};
use crate::diagnostics::{DiagnosticsConverter, diagnostics_utils};
use crate::completion::CompletionEngine;
use crate::hover::HoverEngine;
use crate::goto_definition::GotoDefinitionEngine;
use crate::symbol_index::SymbolIndex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, error, info, warn};

pub struct GrenLspService {
    client: Client,
    document_manager: Arc<RwLock<DocumentManager>>,
    compiler: Arc<GrenCompiler>,
    diagnostics_converter: Arc<RwLock<Option<DiagnosticsConverter>>>,
    /// Workspace root for project resolution
    workspace_root: Arc<RwLock<Option<PathBuf>>>,
    /// Symbol index for cross-module resolution
    symbol_index: Arc<RwLock<Option<SymbolIndex>>>,
    /// Completion engine
    completion_engine: Arc<RwLock<Option<CompletionEngine>>>,
    /// Hover engine
    hover_engine: Arc<RwLock<Option<HoverEngine>>>,
    /// Go-to-definition engine
    goto_definition_engine: Arc<RwLock<Option<GotoDefinitionEngine>>>,
}

impl GrenLspService {
    pub fn new(client: Client) -> Self {
        // Initialize compiler with default config, attempting to use GREN_COMPILER_PATH
        let compiler = match GrenCompiler::with_env() {
            Ok(compiler) => Arc::new(compiler),
            Err(e) => {
                // Log warning but continue with default config
                eprintln!("Warning: Failed to initialize compiler with environment: {}. Using default config.", e);
                Arc::new(GrenCompiler::new(CompilerConfig::default()))
            }
        };

        Self { 
            client,
            document_manager: Arc::new(RwLock::new(DocumentManager::new(100))), // LRU cache of 100 items
            compiler,
            diagnostics_converter: Arc::new(RwLock::new(None)),
            workspace_root: Arc::new(RwLock::new(None)),
            symbol_index: Arc::new(RwLock::new(None)),
            completion_engine: Arc::new(RwLock::new(None)),
            hover_engine: Arc::new(RwLock::new(None)),
            goto_definition_engine: Arc::new(RwLock::new(None)),
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

    /// Initialize workspace and diagnostics converter when workspace root is known
    async fn initialize_workspace(&self, root_path: PathBuf) {
        info!("Initializing workspace at {:?}", root_path);
        
        // Set workspace root
        *self.workspace_root.write().await = Some(root_path.clone());
        
        // Initialize diagnostics converter
        *self.diagnostics_converter.write().await = Some(DiagnosticsConverter::new(root_path.clone()));
        
        // Initialize symbol index
        let db_path = root_path.join(".gren-lsp").join("symbols.db");
        match SymbolIndex::new(&db_path, root_path.clone()).await {
            Ok(symbol_index) => {
                info!("Symbol index initialized successfully");
                
                // Initialize completion engine
                match CompletionEngine::new(symbol_index.clone()) {
                    Ok(completion_engine) => {
                        info!("Completion engine initialized successfully");
                        *self.completion_engine.write().await = Some(completion_engine);
                    }
                    Err(e) => {
                        error!("Failed to initialize completion engine: {}", e);
                    }
                }
                
                // Initialize hover engine
                match HoverEngine::new(symbol_index.clone()) {
                    Ok(hover_engine) => {
                        info!("Hover engine initialized successfully");
                        *self.hover_engine.write().await = Some(hover_engine);
                    }
                    Err(e) => {
                        error!("Failed to initialize hover engine: {}", e);
                    }
                }
                
                // Initialize go-to-definition engine
                match GotoDefinitionEngine::new(symbol_index.clone()) {
                    Ok(goto_definition_engine) => {
                        info!("Go-to-definition engine initialized successfully");
                        *self.goto_definition_engine.write().await = Some(goto_definition_engine);
                    }
                    Err(e) => {
                        error!("Failed to initialize go-to-definition engine: {}", e);
                    }
                }
                
                // Store symbol index after engines are initialized
                *self.symbol_index.write().await = Some(symbol_index);
            }
            Err(e) => {
                error!("Failed to initialize symbol index: {}", e);
            }
        }
    }

    /// Compile a document and publish diagnostics
    async fn compile_and_publish_diagnostics(&self, uri: &Url) {
        let _workspace_root = self.workspace_root.read().await.clone();
        let _workspace_root = match _workspace_root {
            Some(root) => root,
            None => {
                debug!("Cannot compile {}: workspace root not initialized", uri);
                return;
            }
        };

        // Get document content from document manager
        let doc_manager = self.document_manager.read().await;
        let document_content = doc_manager.get_open_document_content(uri);
        drop(doc_manager);

        let document_content = match document_content {
            Some(content) => content,
            None => {
                debug!("Cannot compile {}: document not found", uri);
                return;
            }
        };

        // Convert URI to file path and determine module name
        let file_path = match uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                warn!("Cannot compile {}: invalid file path", uri);
                return;
            }
        };

        // Determine project root by searching upward for gren.json
        let project_root = match crate::compiler_interface::project_utils::find_project_root(&file_path).await {
            Ok(root) => root,
            Err(e) => {
                debug!("Cannot find project root for {}: {}", uri, e);
                return;
            }
        };

        // Determine module name from file path
        let module_name = match crate::compiler_interface::project_utils::module_name_from_path(&file_path, &project_root) {
            Ok(name) => name,
            Err(e) => {
                warn!("Cannot determine module name for {}: {}", uri, e);
                return;
            }
        };

        // Create in-memory documents map for current document
        let mut in_memory_documents = HashMap::new();
        let relative_path = match file_path.strip_prefix(&project_root) {
            Ok(rel) => rel.to_path_buf(),
            Err(_) => {
                warn!("File {} is not within project root {:?}", uri, project_root);
                return;
            }
        };
        in_memory_documents.insert(relative_path, document_content);

        // Create compilation request
        let compile_request = CompileRequest {
            module_name,
            project_root,
            include_sourcemaps: false,
            in_memory_documents,
        };

        // Compile the module
        let compile_result = match self.compiler.compile(compile_request).await {
            Ok(result) => result,
            Err(e) => {
                error!("Compilation failed for {}: {}", uri, e);
                return;
            }
        };

        // Parse compiler output into diagnostics
        let diagnostics = if compile_result.success {
            // No errors - clear diagnostics
            HashMap::new()
        } else {
            // Parse errors into diagnostics
            match self.compiler.parse_compiler_output(&compile_result.output) {
                Ok(Some(compiler_output)) => {
                    let converter = self.diagnostics_converter.read().await;
                    match converter.as_ref() {
                        Some(converter) => {
                            match converter.convert_to_diagnostics(&compiler_output) {
                                Ok(diags) => diags,
                                Err(e) => {
                                    error!("Failed to convert compiler output to diagnostics: {}", e);
                                    HashMap::new()
                                }
                            }
                        }
                        None => {
                            error!("Diagnostics converter not initialized");
                            HashMap::new()
                        }
                    }
                }
                Ok(None) => HashMap::new(),
                Err(e) => {
                    error!("Failed to parse compiler output: {}", e);
                    HashMap::new()
                }
            }
        };

        // Log diagnostics summary
        let (errors, warnings, info, hints) = diagnostics_utils::count_by_severity(&diagnostics);
        debug!("Diagnostics for {}: {} errors, {} warnings, {} info, {} hints", 
               uri, errors, warnings, info, hints);

        // Check if current file has diagnostics before consuming the map
        let has_diagnostics_for_current_file = diagnostics.contains_key(uri);

        // Publish diagnostics for each file
        for (file_uri, file_diagnostics) in diagnostics {
            self.client.publish_diagnostics(file_uri, file_diagnostics, None).await;
        }

        // If no diagnostics for the current file, clear its diagnostics
        if !has_diagnostics_for_current_file {
            self.client.publish_diagnostics(uri.clone(), vec![], None).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for GrenLspService {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("LSP initialize request received");
        
        // Initialize workspace root if provided
        if let Some(root_uri) = params.root_uri {
            if let Ok(root_path) = root_uri.to_file_path() {
                self.initialize_workspace(root_path).await;
            } else {
                warn!("Invalid root URI provided: {}", root_uri);
            }
        } else if let Some(root_path) = params.root_path {
            self.initialize_workspace(PathBuf::from(root_path)).await;
        }
        
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
            drop(doc_manager);
            
            // Trigger compilation and diagnostics for opened document
            self.compile_and_publish_diagnostics(&uri).await;
            
            // Index symbols from the opened document
            self.index_document_symbols(&uri).await;
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
        } else {
            drop(doc_manager);
            
            // Trigger compilation and diagnostics for changed document
            self.compile_and_publish_diagnostics(&uri).await;
            
            // Re-index symbols from the changed document
            self.index_document_symbols(&uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        info!("Document saved: {}", uri);
        
        // Trigger compilation and diagnostics for saved document
        self.compile_and_publish_diagnostics(&uri).await;
        
        // Re-index symbols from the saved document
        self.index_document_symbols(&uri).await;
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
            drop(doc_manager);
            
            // Clear diagnostics for closed document
            self.client.publish_diagnostics(uri, vec![], None).await;
        }
    }

    // Language feature implementations
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        info!("Hover request at {:?}", params.text_document_position_params.position);
        
        let uri = &params.text_document_position_params.text_document.uri;
        
        // Get document content
        let doc_manager = self.document_manager.read().await;
        let document_content = match doc_manager.get_open_document_content(uri) {
            Some(content) => content,
            None => {
                debug!("Document not found for hover: {}", uri);
                return Ok(None);
            }
        };
        drop(doc_manager);
        
        // Get hover engine
        let mut hover_engine = self.hover_engine.write().await;
        let hover_engine = match hover_engine.as_mut() {
            Some(engine) => engine,
            None => {
                debug!("Hover engine not initialized");
                return Ok(None);
            }
        };
        
        // Handle hover request
        match hover_engine.handle_hover(params, &document_content).await {
            Ok(response) => {
                if response.is_some() {
                    debug!("Hover returned information");
                } else {
                    debug!("Hover returned no information");
                }
                Ok(response)
            }
            Err(e) => {
                error!("Hover failed: {}", e);
                Ok(None)
            }
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        info!("Completion request at {:?}", params.text_document_position.position);
        
        let uri = &params.text_document_position.text_document.uri;
        
        // Get document content
        let doc_manager = self.document_manager.read().await;
        let document_content = match doc_manager.get_open_document_content(uri) {
            Some(content) => content,
            None => {
                debug!("Document not found for completion: {}", uri);
                return Ok(None);
            }
        };
        drop(doc_manager);
        
        // Get completion engine
        let completion_engine = self.completion_engine.read().await;
        let completion_engine = match completion_engine.as_ref() {
            Some(engine) => engine,
            None => {
                debug!("Completion engine not initialized");
                return Ok(None);
            }
        };
        
        // Handle completion request
        match completion_engine.handle_completion(params, &document_content).await {
            Ok(response) => {
                if let Some(CompletionResponse::Array(ref items)) = response {
                    debug!("Completion returned {} items", items.len());
                } else {
                    debug!("Completion returned no items");
                }
                Ok(response)
            }
            Err(e) => {
                error!("Completion failed: {}", e);
                Ok(None)
            }
        }
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        info!("Go to definition request at {:?}", params.text_document_position_params.position);
        
        let uri = &params.text_document_position_params.text_document.uri;
        
        // Get document content
        let doc_manager = self.document_manager.read().await;
        let document_content = match doc_manager.get_open_document_content(uri) {
            Some(content) => content,
            None => {
                debug!("Document not found for go-to-definition: {}", uri);
                return Ok(None);
            }
        };
        drop(doc_manager);
        
        // Get go-to-definition engine
        let mut goto_definition_engine = self.goto_definition_engine.write().await;
        let goto_definition_engine = match goto_definition_engine.as_mut() {
            Some(engine) => engine,
            None => {
                debug!("Go-to-definition engine not initialized");
                return Ok(None);
            }
        };
        
        // Handle go-to-definition request
        match goto_definition_engine.handle_goto_definition(params, &document_content).await {
            Ok(response) => {
                if response.is_some() {
                    debug!("Go-to-definition found target location");
                } else {
                    debug!("Go-to-definition found no target");
                }
                Ok(response)
            }
            Err(e) => {
                error!("Go-to-definition failed: {}", e);
                Ok(None)
            }
        }
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

impl GrenLspService {
    /// Index symbols from a document
    async fn index_document_symbols(&self, uri: &Url) {
        let symbol_index = self.symbol_index.read().await;
        let symbol_index = match symbol_index.as_ref() {
            Some(index) => index,
            None => {
                debug!("Symbol index not initialized, skipping indexing for {}", uri);
                return;
            }
        };

        // Get document content
        let doc_manager = self.document_manager.read().await;
        let document_content = match doc_manager.get_open_document_content(uri) {
            Some(content) => content,
            None => {
                debug!("Document not found for indexing: {}", uri);
                return;
            }
        };
        drop(doc_manager);

        // Index the document
        match symbol_index.index_file(uri, &document_content).await {
            Ok(_) => {
                debug!("Successfully indexed symbols for {}", uri);
            }
            Err(e) => {
                error!("Failed to index symbols for {}: {}", uri, e);
            }
        }
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