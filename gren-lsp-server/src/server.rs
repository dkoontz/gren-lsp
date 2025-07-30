use gren_lsp_core::Workspace;
use gren_lsp_protocol::handlers::Handlers;
use lsp_types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer};
use tracing::{error, info, warn};

pub struct GrenLanguageServer {
    client: Client,
    workspace: Arc<RwLock<Workspace>>,
    client_capabilities: Arc<RwLock<Option<ClientCapabilities>>>,
    debug_export_dir: Option<PathBuf>,
    // Debouncing mechanism for real-time compilation
    pending_diagnostics: Arc<RwLock<HashMap<Url, Instant>>>,
}

impl GrenLanguageServer {
    pub fn new(client: Client) -> Self {
        Self::new_with_debug(client, None)
    }

    pub fn new_with_debug(client: Client, debug_export_dir: Option<PathBuf>) -> Self {
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

        if let Some(ref debug_dir) = debug_export_dir {
            info!("Debug export directory configured: {}", debug_dir.display());
            // Create the debug directory if it doesn't exist
            if let Err(e) = std::fs::create_dir_all(debug_dir) {
                error!("Failed to create debug export directory: {}", e);
            }
        }

        info!("Language server initialization complete");
        Self {
            client,
            workspace: Arc::new(RwLock::new(workspace)),
            client_capabilities: Arc::new(RwLock::new(None)),
            debug_export_dir,
            pending_diagnostics: Arc::new(RwLock::new(HashMap::new())),
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
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
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

        // Get comprehensive diagnostics for the newly opened document (syntax + compiler)
        let diagnostics = match workspace.get_document_diagnostics(&uri).await {
            Ok(diags) => {
                info!(
                    "Found {} comprehensive diagnostics for document: {}",
                    diags.len(),
                    uri
                );
                diags
            }
            Err(e) => {
                warn!("Failed to get comprehensive diagnostics for {}: {}", uri, e);
                // Fallback to syntax-only diagnostics
                let syntax_diagnostics = workspace.get_diagnostics(&uri);
                info!(
                    "Fallback: {} syntax diagnostics for document: {}",
                    syntax_diagnostics.len(),
                    uri
                );
                syntax_diagnostics
            }
        };

        // Export parse tree if debug mode is enabled
        if let Some(ref debug_dir) = self.debug_export_dir {
            if let Err(e) = workspace.export_parse_tree_for_document(&uri, debug_dir) {
                error!("Failed to export parse tree for {}: {}", uri, e);
            }
        }

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

        // Check if this is a project configuration file that should invalidate cache
        if workspace.is_project_file(&uri) {
            info!("🔧 Project configuration file changed: {}", uri);
            workspace.invalidate_compiler_cache();
        }

        // Release the workspace lock before calling debounced diagnostics
        drop(workspace);

        // Export parse tree if debug mode is enabled
        if let Some(ref debug_dir) = self.debug_export_dir {
            let workspace = self.workspace.read().await;
            if let Err(e) = workspace.export_parse_tree_for_document(&uri, debug_dir) {
                error!("Failed to export parse tree for {}: {}", uri, e);
            }
        }

        // Schedule debounced diagnostics update
        self.schedule_debounced_diagnostics(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        info!("Document saved: {}", params.text_document.uri);

        let uri = params.text_document.uri.clone();
        let mut workspace = self.workspace.write().await;

        // Force refresh diagnostics after save (bypasses cache)
        let diagnostics = match workspace.force_refresh_diagnostics(&uri).await {
            Ok(diags) => {
                info!(
                    "Found {} diagnostics after save for: {}",
                    diags.len(),
                    uri
                );
                diags
            }
            Err(e) => {
                warn!("Failed to get diagnostics after save for {}: {}", uri, e);
                // Fallback to syntax-only diagnostics
                workspace.get_diagnostics(&uri)
            }
        };

        // Publish fresh diagnostics
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

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        info!(
            "Workspace folders changed: {} added, {} removed",
            params.event.added.len(),
            params.event.removed.len()
        );

        // Handle removed workspace folders
        for removed_folder in &params.event.removed {
            info!("Removing workspace folder: {}", removed_folder.uri);
            self.cleanup_workspace_folder(&removed_folder.uri).await;
        }

        // Handle added workspace folders
        for added_folder in &params.event.added {
            info!("Adding workspace folder: {}", added_folder.uri);
            self.index_workspace_folder(&added_folder.uri).await;
        }

        info!("Workspace folder changes processed");
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

        // Create progress reporting
        let progress_token = self.create_progress("Indexing workspace").await;

        let workspace_root = {
            let workspace = self.workspace.read().await;
            workspace.stats().root_uri.clone()
        };

        if let Some(root_uri) = workspace_root {
            info!("Indexing workspace at: {}", root_uri);

            if let Some(token) = &progress_token {
                self.report_progress(token, "Discovering files...", Some(10))
                    .await;
            }

            // Use the client's file search capabilities instead of filesystem crawling
            // This respects .gitignore and other editor exclusion rules
            if let Ok(root_path) = root_uri.to_file_path() {
                self.discover_and_index_files_with_progress(root_path, progress_token.as_deref())
                    .await;
            }
        } else {
            info!("No workspace root set, skipping indexing");
        }

        // End progress reporting
        if let Some(token) = progress_token {
            self.end_progress(&token, Some("Indexing completed")).await;
        }
    }

    /// Discover Gren files using basic filesystem traversal as fallback
    /// In a production implementation, we'd prefer to use workspace/symbol
    /// or other LSP client capabilities for file discovery
    async fn discover_and_index_files(&self, root_path: std::path::PathBuf) {
        self.discover_and_index_files_with_progress(root_path, None)
            .await;
    }

    /// Discover and index files with optional progress reporting
    async fn discover_and_index_files_with_progress(
        &self,
        root_path: std::path::PathBuf,
        progress_token: Option<&str>,
    ) {
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

        if let Some(token) = progress_token {
            self.report_progress(token, "Scanning directories...", Some(20))
                .await;
        }

        let mut gren_files = Vec::new();
        if let Err(e) = walk_dir(&root_path, &mut gren_files).await {
            error!("Failed to walk directory {}: {}", root_path.display(), e);
            return;
        }

        info!("Found {} Gren files to index", gren_files.len());

        if let Some(token) = progress_token {
            self.report_progress(
                token,
                &format!("Indexing {} files...", gren_files.len()),
                Some(30),
            )
            .await;
        }

        let total_files = gren_files.len();

        // Index files with progress updates
        for (index, file_path) in gren_files.iter().enumerate() {
            if let Ok(uri) = Url::from_file_path(file_path) {
                self.index_file(&uri).await;
            }

            // Report progress every 10 files or for the last file
            if let Some(token) = progress_token {
                if index % 10 == 0 || index == total_files - 1 {
                    let percentage = 30 + ((index + 1) * 60 / total_files.max(1)) as u32;
                    let message = format!("Indexed {}/{} files", index + 1, total_files);
                    self.report_progress(token, &message, Some(percentage))
                        .await;
                }
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

    /// Index all files in a specific workspace folder
    async fn index_workspace_folder(&self, folder_uri: &Url) {
        info!("Indexing workspace folder: {}", folder_uri);

        // Create progress reporting for workspace folder indexing
        let progress_token = self
            .create_progress(&format!("Indexing folder: {}", folder_uri.path()))
            .await;

        if let Ok(folder_path) = folder_uri.to_file_path() {
            self.discover_and_index_files_with_progress(folder_path, progress_token.as_deref())
                .await;
        } else {
            error!("Invalid workspace folder path: {}", folder_uri);
        }

        // End progress reporting
        if let Some(token) = progress_token {
            self.end_progress(&token, Some("Folder indexing completed"))
                .await;
        }
    }

    /// Clean up symbols and documents from a removed workspace folder
    async fn cleanup_workspace_folder(&self, folder_uri: &Url) {
        info!("Cleaning up workspace folder: {}", folder_uri);

        let folder_path = match folder_uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                error!("Invalid workspace folder path: {}", folder_uri);
                return;
            }
        };

        let folder_path_str = folder_path.to_string_lossy();

        // Get list of documents that start with the folder path
        let documents_to_remove = {
            let workspace = self.workspace.read().await;
            let stats = workspace.stats();
            let mut docs = Vec::new();

            // Find all documents in this workspace folder
            // This is a simplified approach - in a more sophisticated implementation,
            // we'd have better workspace folder tracking
            for uri in stats.open_documents {
                if let Ok(doc_path) = uri.to_file_path() {
                    let doc_path_str = doc_path.to_string_lossy();
                    if doc_path_str.starts_with(&*folder_path_str) {
                        docs.push(uri);
                    }
                }
            }
            docs
        };

        // Close documents from the removed workspace folder
        let mut workspace = self.workspace.write().await;
        for uri in documents_to_remove {
            info!("Removing document from workspace: {}", uri);
            if let Err(e) = workspace.close_document(uri.clone()) {
                error!("Failed to remove document {}: {}", uri, e);
            }
        }

        info!("Workspace folder cleanup completed: {}", folder_uri);
    }

    /// Create a work done progress token and begin progress reporting
    async fn create_progress(&self, title: &str) -> Option<String> {
        // Check if client supports work done progress
        let supports_progress = {
            let capabilities = self.client_capabilities.read().await;
            capabilities
                .as_ref()
                .and_then(|caps| caps.window.as_ref())
                .and_then(|window| window.work_done_progress)
                .unwrap_or(false)
        };

        if !supports_progress {
            info!("Client doesn't support work done progress, using log messages");
            return None;
        }

        // For now, use basic logging until we implement proper progress
        // TODO: Implement proper LSP work done progress when tower-lsp supports it
        info!("Starting progress: {}", title);
        Some(title.to_string())
    }

    /// Report progress update
    async fn report_progress(&self, _token: &str, message: &str, percentage: Option<u32>) {
        if let Some(pct) = percentage {
            info!("Progress ({}%): {}", pct, message);
        } else {
            info!("Progress: {}", message);
        }
    }

    /// End progress reporting
    async fn end_progress(&self, _token: &str, message: Option<&str>) {
        if let Some(msg) = message {
            info!("Progress completed: {}", msg);
        } else {
            info!("Progress completed");
        }
    }

    /// Schedule debounced diagnostics update for real-time feedback
    async fn schedule_debounced_diagnostics(&self, uri: Url) {
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(500); // 500ms debounce
        
        // Record when this document was last changed
        {
            let mut pending = self.pending_diagnostics.write().await;
            pending.insert(uri.clone(), Instant::now());
        }
        
        // Clone necessary data for the async task
        let uri_clone = uri.clone();
        let client = self.client.clone();
        let workspace = self.workspace.clone();
        let pending_diagnostics = self.pending_diagnostics.clone();
        
        // Spawn a task to handle the debounced update
        tokio::spawn(async move {
            // Wait for the debounce period
            sleep(DEBOUNCE_DURATION).await;
            
            // Check if this is still the latest change for this document
            let should_process = {
                let pending = pending_diagnostics.read().await;
                if let Some(last_change) = pending.get(&uri_clone) {
                    // Only process if enough time has passed since the last change
                    last_change.elapsed() >= DEBOUNCE_DURATION
                } else {
                    false // Document was removed from pending updates
                }
            };
            
            if should_process {
                info!("⏰ Processing debounced diagnostics for: {}", uri_clone);
                
                // Remove from pending updates
                {
                    let mut pending = pending_diagnostics.write().await;
                    pending.remove(&uri_clone);
                }
                
                // Get comprehensive diagnostics (syntax + compiler)
                let diagnostics = {
                    let mut workspace = workspace.write().await;
                    match workspace.get_document_diagnostics(&uri_clone).await {
                        Ok(diags) => {
                            info!(
                                "Found {} real-time diagnostics for: {}",
                                diags.len(),
                                uri_clone
                            );
                            diags
                        }
                        Err(e) => {
                            warn!("Failed to get real-time diagnostics for {}: {}", uri_clone, e);
                            // Fallback to syntax-only diagnostics
                            let syntax_diagnostics = workspace.get_diagnostics(&uri_clone);
                            info!(
                                "Fallback: {} syntax diagnostics for: {}",
                                syntax_diagnostics.len(),
                                uri_clone
                            );
                            syntax_diagnostics
                        }
                    }
                };
                
                // Publish diagnostics
                client.publish_diagnostics(uri_clone, diagnostics, None).await;
            } else {
                info!("⚡ Skipping outdated diagnostic update for: {}", uri_clone);
            }
        });
    }
}
