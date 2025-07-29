use gren_lsp_core::Workspace;
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
    client_capabilities: Arc<RwLock<Option<ClientCapabilities>>>,
}

impl GrenLanguageServer {
    pub fn new(client: Client) -> Self {
        info!("Initializing language server");

        // Initialize workspace with error handling
        let workspace = match Workspace::new() {
            Ok(ws) => {
                info!("Workspace initialized successfully");
                ws
            }
            Err(e) => {
                error!("Failed to initialize workspace: {}", e);
                panic!("Failed to create workspace: {}", e)
            }
        };

        info!("Language server initialization complete");
        Self {
            client,
            workspace: Arc::new(RwLock::new(workspace)),
            client_capabilities: Arc::new(RwLock::new(None)),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for GrenLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Initialize request received");

        // Store client capabilities
        {
            let mut capabilities = self.client_capabilities.write().await;
            *capabilities = Some(params.capabilities.clone());
        }

        // Set workspace root - prefer workspaceFolders, fallback to rootUri
        let workspace_folders = params.workspace_folders.clone();
        if let Some(folders) = &workspace_folders {
            if !folders.is_empty() {
                let mut workspace = self.workspace.write().await;
                // Use the first workspace folder as the primary root
                workspace.set_root(folders[0].uri.clone());
                info!("Set workspace root to: {}", folders[0].uri);
            }
        } else if let Some(root_uri) = params.root_uri {
            let mut workspace = self.workspace.write().await;
            workspace.set_root(root_uri.clone());
            info!("Set workspace root to: {}", root_uri);
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
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                // TODO: Implement these features
                // references_provider: Some(OneOf::Left(true)),
                // code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                // rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("Server initialized");

        // Register file watchers for Gren files according to LSP spec
        self.register_file_watchers().await;

        // Index any existing Gren files in the workspace
        self.index_workspace_files().await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutdown request received");
        Ok(())
    }

    // Document synchronization
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        info!("Document opened: {}", params.text_document.uri);

        let uri = params.text_document.uri.clone();
        let mut workspace = self.workspace.write().await;

        if let Err(e) = workspace.open_document(params.text_document) {
            error!("Failed to open document: {}", e);
            return;
        }

        // Get diagnostics for the newly opened document
        let diagnostics = workspace.get_diagnostics(&uri);
        info!(
            "Found {} diagnostics for document: {}",
            diagnostics.len(),
            uri
        );

        // Log workspace stats
        let stats = workspace.stats();
        info!("Workspace stats: {} documents open", stats.document_count);

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        info!("Document changed: {} (version {})", uri, version);

        let mut workspace = self.workspace.write().await;

        if let Err(e) = workspace.update_document(params) {
            error!("Failed to update document {}: {}", uri, e);
            return;
        }

        // Get updated diagnostics for the changed document
        let diagnostics = workspace.get_diagnostics(&uri);
        info!(
            "Found {} diagnostics for changed document: {}",
            diagnostics.len(),
            uri
        );

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        info!("Document closed: {}", params.text_document.uri);

        let uri = params.text_document.uri.clone();
        let mut workspace = self.workspace.write().await;

        if let Err(e) = workspace.close_document(params.text_document.uri) {
            error!("Failed to close document: {}", e);
        }

        // Clear diagnostics for closed document
        self.client.publish_diagnostics(uri, Vec::new(), None).await;

        // Log workspace stats
        let stats = workspace.stats();
        info!("Workspace stats: {} documents open", stats.document_count);
    }

    // Language features
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let handlers = Handlers::new(self.workspace.clone());
        let client_capabilities = self.client_capabilities.read().await;
        handlers
            .hover_with_capabilities(params, client_capabilities.as_ref())
            .await
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

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let handlers = Handlers::new(self.workspace.clone());
        handlers.document_symbols(params).await
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let handlers = Handlers::new(self.workspace.clone());
        handlers.workspace_symbols(params).await
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        info!("Watched files changed: {} files", params.changes.len());

        for change in params.changes {
            info!("File {} changed: {:?}", change.uri, change.typ);

            match change.typ {
                FileChangeType::CREATED | FileChangeType::CHANGED => {
                    // For created or changed files, try to index them
                    self.index_file(&change.uri).await;
                }
                FileChangeType::DELETED => {
                    // For deleted files, remove them from our index
                    let mut workspace = self.workspace.write().await;
                    if let Err(e) = workspace.close_document(change.uri.clone()) {
                        info!("File was not open, removing from index: {}", e);
                    }
                }
                _ => {}
            }
        }
    }
}

impl GrenLanguageServer {
    /// Register file watchers for Gren files using LSP client capabilities
    async fn register_file_watchers(&self) {
        info!("Registering file watchers for *.gren files");

        let registration_params = RegistrationParams {
            registrations: vec![Registration {
                id: "gren-file-watcher".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: Some(
                    serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                        watchers: vec![FileSystemWatcher {
                            glob_pattern: GlobPattern::String("**/*.gren".to_string()),
                            kind: None, // Default to all kinds (CREATE | CHANGE | DELETE)
                        }],
                    })
                    .unwrap(),
                ),
            }],
        };

        if let Err(e) = self
            .client
            .register_capability(registration_params.registrations)
            .await
        {
            error!("Failed to register file watchers: {}", e);
        } else {
            info!("Successfully registered file watchers");
        }
    }

    /// Index all existing Gren files in the workspace
    async fn index_workspace_files(&self) {
        info!("Starting workspace indexing");

        let workspace_root = {
            let workspace = self.workspace.read().await;
            workspace.stats().root_uri.clone()
        };

        if let Some(root_uri) = workspace_root {
            info!("Indexing workspace at: {}", root_uri);

            // Use the client's file search capabilities instead of filesystem crawling
            // This respects .gitignore and other editor exclusion rules
            if let Ok(root_path) = root_uri.to_file_path() {
                self.discover_and_index_files(root_path).await;
            }
        } else {
            info!("No workspace root set, skipping indexing");
        }
    }

    /// Discover Gren files using basic filesystem traversal as fallback
    /// In a production implementation, we'd prefer to use workspace/symbol
    /// or other LSP client capabilities for file discovery
    async fn discover_and_index_files(&self, root_path: std::path::PathBuf) {
        use std::path::Path;
        use tokio::fs;

        async fn walk_dir(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
            let mut entries = fs::read_dir(dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_dir() {
                    // Skip common ignored directories
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if matches!(
                            name,
                            ".git" | "node_modules" | "target" | ".vscode" | ".idea"
                        ) {
                            continue;
                        }
                    }
                    Box::pin(walk_dir(&path, files)).await?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("gren") {
                    files.push(path);
                }
            }
            Ok(())
        }

        let mut gren_files = Vec::new();
        if let Err(e) = walk_dir(&root_path, &mut gren_files).await {
            error!("Failed to walk directory {}: {}", root_path.display(), e);
            return;
        }

        info!("Found {} Gren files to index", gren_files.len());

        // Index files in batches to avoid overwhelming the system
        for file_path in gren_files {
            if let Ok(uri) = Url::from_file_path(&file_path) {
                self.index_file(&uri).await;
            }
        }

        info!("Workspace indexing complete");
    }

    /// Index a single file by URI
    async fn index_file(&self, uri: &Url) {
        use tokio::fs;

        // Read file content
        let file_path = match uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                error!("Invalid file path: {}", uri);
                return;
            }
        };

        let content = match fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read file {}: {}", uri, e);
                return;
            }
        };

        // Create a TextDocumentItem for the file
        let text_document = TextDocumentItem {
            uri: uri.clone(),
            language_id: "gren".to_string(),
            version: 1,
            text: content,
        };

        // Add to workspace
        let mut workspace = self.workspace.write().await;
        if let Err(e) = workspace.open_document(text_document) {
            error!("Failed to index file {}: {}", uri, e);
        } else {
            info!("Successfully indexed file: {}", uri);
        }
    }
}
