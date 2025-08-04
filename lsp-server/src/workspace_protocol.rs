use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tracing::{debug, error, info, warn};

use crate::module_rename::{ModuleRenameEngine, ModuleRenameRequest};

/// Handles LSP workspace protocol operations
pub struct WorkspaceProtocolHandler {
    /// Module rename engine
    module_rename_engine: Arc<RwLock<Option<ModuleRenameEngine>>>,
}

impl WorkspaceProtocolHandler {
    /// Create new workspace protocol handler
    pub fn new() -> Self {
        Self {
            module_rename_engine: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize with module rename engine
    pub async fn initialize(&self, module_rename_engine: ModuleRenameEngine) {
        *self.module_rename_engine.write().await = Some(module_rename_engine);
        debug!("Workspace protocol handler initialized");
    }

    /// Handle workspace/willRenameFiles request
    /// LSP Protocol: Only return workspace edits for text changes, no file operations
    pub async fn handle_will_rename_files(
        &self,
        params: RenameFilesParams,
        workspace_documents: &HashMap<Url, String>,
    ) -> Result<Option<WorkspaceEdit>> {
        info!("🔄 Processing willRenameFiles with {} files", params.files.len());

        let module_rename_engine = self.module_rename_engine.read().await;
        let module_rename_engine = match module_rename_engine.as_ref() {
            Some(engine) => engine,
            None => {
                warn!("Module rename engine not initialized");
                return Ok(None);
            }
        };

        let mut all_edits = HashMap::new();

        // Process each file operation
        for file_operation in &params.files {
            let old_uri = Url::parse(&file_operation.old_uri)
                .map_err(|e| anyhow!("Invalid old URI: {}", e))?;
            let new_uri = Url::parse(&file_operation.new_uri)
                .map_err(|e| anyhow!("Invalid new URI: {}", e))?;

            // Check if this is a Gren file
            if !self.is_gren_file(&old_uri) {
                debug!("Skipping non-Gren file: {}", old_uri);
                continue;
            }

            debug!("Processing Gren file rename: {} -> {}", old_uri, new_uri);

            // Create rename request
            let rename_request = ModuleRenameRequest {
                old_uri: old_uri.clone(),
                new_uri: new_uri.clone(),
                workspace_documents: workspace_documents.clone(),
            };

            // Prepare workspace edits (LSP Protocol compliant - no file operations)
            match module_rename_engine.prepare_rename_edits(&rename_request).await {
                Ok(workspace_edit) => {
                    // Merge workspace edits
                    if let Some(changes) = workspace_edit.changes {
                        for (uri, edits) in changes {
                            all_edits.entry(uri)
                                .or_insert_with(Vec::new)
                                .extend(edits);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to prepare rename edits for {}: {}", old_uri, e);
                    continue;
                }
            }
        }

        let workspace_edit = if all_edits.is_empty() {
            None
        } else {
            Some(WorkspaceEdit {
                changes: Some(all_edits),
                document_changes: None,
                change_annotations: None,
            })
        };

        if let Some(ref edit) = workspace_edit {
            let change_count = edit.changes.as_ref().map_or(0, |c| c.values().map(|v| v.len()).sum::<usize>());
            info!("✅ willRenameFiles completed with {} text edits across {} files", 
                change_count, 
                edit.changes.as_ref().map_or(0, |c| c.len()));
        } else {
            info!("willRenameFiles completed with no changes");
        }

        Ok(workspace_edit)
    }

    /// Handle workspace/didRenameFiles notification
    /// LSP Protocol: Called after the editor has completed the file operations
    pub async fn handle_did_rename_files(
        &self,
        params: RenameFilesParams,
        workspace_documents: &HashMap<Url, String>,
    ) -> Result<()> {
        info!("📁 Processing didRenameFiles with {} files", params.files.len());

        let module_rename_engine = self.module_rename_engine.read().await;
        if let Some(engine) = module_rename_engine.as_ref() {
            // Process each file operation for finalization
            for file_operation in &params.files {
                let old_uri = Url::parse(&file_operation.old_uri)
                    .map_err(|e| anyhow!("Invalid old URI: {}", e))?;
                let new_uri = Url::parse(&file_operation.new_uri)
                    .map_err(|e| anyhow!("Invalid new URI: {}", e))?;

                if self.is_gren_file(&old_uri) {
                    debug!("Finalizing Gren file rename: {} -> {}", old_uri, new_uri);
                    
                    // Create rename request for finalization
                    let rename_request = ModuleRenameRequest {
                        old_uri: old_uri.clone(),
                        new_uri: new_uri.clone(),
                        workspace_documents: workspace_documents.clone(),
                    };

                    // Finalize the rename operation (update internal state, etc.)
                    if let Err(e) = engine.finalize_rename(&rename_request).await {
                        error!("Failed to finalize rename for {}: {}", old_uri, e);
                    } else {
                        info!("Gren file rename finalized: {}", new_uri);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a URI refers to a Gren file
    fn is_gren_file(&self, uri: &Url) -> bool {
        if let Ok(path) = uri.to_file_path() {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("gren"))
                .unwrap_or(false)
        } else {
            false
        }
    }
}

/// LSP workspace file operations support
pub trait WorkspaceFileOperations {
    /// Check if the server supports will/didRenameFiles operations
    fn supports_file_operations() -> bool {
        true
    }

    /// Get file operation filters for workspace registration
    fn get_file_operation_filters() -> Vec<FileOperationFilter> {
        vec![
            FileOperationFilter {
                scheme: Some("file".to_string()),
                pattern: FileOperationPattern {
                    glob: "**/*.gren".to_string(),
                    matches: Some(FileOperationPatternKind::File),
                    options: Some(FileOperationPatternOptions {
                        ignore_case: Some(false),
                    }),
                },
            }
        ]
    }

    /// Create workspace file operation registration
    fn create_file_operation_registration() -> Registration {
        Registration {
            id: "workspace-file-operations".to_string(),
            method: "workspace/willRenameFiles".to_string(),
            register_options: Some(serde_json::to_value(FileOperationRegistrationOptions {
                filters: Self::get_file_operation_filters(),
            }).unwrap()),
        }
    }
}

impl WorkspaceFileOperations for WorkspaceProtocolHandler {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_is_gren_file() {
        let handler = WorkspaceProtocolHandler::new();

        // Test Gren files
        let gren_uri = Url::parse("file:///path/to/Module.gren").unwrap();
        assert!(handler.is_gren_file(&gren_uri));

        // Test non-Gren files
        let js_uri = Url::parse("file:///path/to/file.js").unwrap();
        assert!(!handler.is_gren_file(&js_uri));

        let no_ext_uri = Url::parse("file:///path/to/file").unwrap();
        assert!(!handler.is_gren_file(&no_ext_uri));
    }

    #[test]
    fn test_file_operation_filters() {
        let filters = WorkspaceProtocolHandler::get_file_operation_filters();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].pattern.glob, "**/*.gren");
        assert_eq!(filters[0].scheme, Some("file".to_string()));
    }

    #[tokio::test]
    async fn test_handle_will_rename_files_no_engine() {
        let handler = WorkspaceProtocolHandler::new();
        let workspace_docs = HashMap::new();

        let params = RenameFilesParams {
            files: vec![FileRename {
                old_uri: "file:///old.gren".to_string(),
                new_uri: "file:///new.gren".to_string(),
            }],
        };

        let result = handler.handle_will_rename_files(params, &workspace_docs).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_handle_did_rename_files_empty() {
        let handler = WorkspaceProtocolHandler::new();

        let params = RenameFilesParams {
            files: vec![],
        };

        let workspace_documents = std::collections::HashMap::new();
        let result = handler.handle_did_rename_files(params, &workspace_documents).await;
        assert!(result.is_ok());
    }
}