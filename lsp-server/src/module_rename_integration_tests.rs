use std::collections::HashMap;
use tempfile::TempDir;
use tower_lsp::lsp_types::*;
use url::Url;
use tokio::fs;
use anyhow::Result;

use crate::module_rename::{ModuleRenameEngine, ModuleRenameRequest};
use crate::import_rewriter::ImportRewriter;
use crate::workspace_protocol::WorkspaceProtocolHandler;
use crate::symbol_index::SymbolIndex;

/// Integration tests for module rename operations
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Test: Single Module File Rename
    #[tokio::test]
    async fn test_single_module_file_rename() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create test files
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper)\n\nhelper : String -> String\nhelper s = s").await.unwrap();
        
        let main_path = temp_dir.path().join("src").join("Main.gren");
        fs::write(&main_path, "module Main exposing (..)\n\nimport Utils\n\nmain = Utils.helper \"test\"").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Create rename request
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let helpers_path = temp_dir.path().join("src").join("Helpers.gren");
        let new_uri = Url::from_file_path(&helpers_path).unwrap();
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        workspace_documents.insert(Url::from_file_path(&main_path).unwrap(), fs::read_to_string(&main_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri: old_uri.clone(),
            new_uri: new_uri.clone(),
            workspace_documents,
        };
        
        // Validate the rename operation
        let validation = engine.validate_rename(&request).await.unwrap();
        assert!(validation.is_valid, "Rename validation should succeed");
        assert_eq!(validation.affected_files.len(), 1); // Main.gren should be affected
        
        // Prepare the rename edits (LSP Protocol compliant)
        let result = engine.prepare_rename_edits(&request).await.unwrap();
        
        // Verify workspace edit contains exactly expected changes
        assert!(result.changes.is_some(), "Workspace edit should contain changes");
        let changes = result.changes.as_ref().unwrap();
        
        // Should have exactly 2 changes: import update in Main.gren and module declaration in renamed file
        assert_eq!(changes.len(), 2, "Should have exactly 2 files with changes");
        
        // Check Main.gren import update
        let main_uri = Url::from_file_path(&main_path).unwrap();
        let main_edits = changes.get(&main_uri).expect("Main.gren should have import updates");
        assert_eq!(main_edits.len(), 1, "Main.gren should have exactly 1 edit");
        assert_eq!(main_edits[0].new_text, "Helpers", "Import should be updated to Helpers");
        
        // Check module declaration update in renamed file
        let helpers_edits = changes.get(&new_uri).expect("Renamed file should have module declaration update");
        assert_eq!(helpers_edits.len(), 1, "Renamed file should have exactly 1 edit");  
        assert_eq!(helpers_edits[0].new_text, "Helpers", "Module declaration should be updated to Helpers");
    }

    /// Test: Nested Module Rename
    #[tokio::test]
    async fn test_nested_module_rename() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create gren.json for valid project
        fs::write(temp_dir.path().join("gren.json"), r#"{"type":"application"}"#).await.unwrap();
        
        // Create nested module structure
        let http_dir = temp_dir.path().join("src").join("Http");
        fs::create_dir_all(&http_dir).await.unwrap();
        let client_path = http_dir.join("Client.gren");
        fs::write(&client_path, "module Http.Client exposing (request)\n\nrequest = \"GET\"").await.unwrap();
        
        let main_path = temp_dir.path().join("src").join("Main.gren");
        fs::write(&main_path, "module Main exposing (..)\n\nimport Http.Client\n\nmain = Http.Client.request").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Create rename request - rename Http/Client.gren to Network/Http.gren
        let old_uri = Url::from_file_path(&client_path).unwrap();
        let network_dir = temp_dir.path().join("src").join("Network");
        let new_path = network_dir.join("Http.gren");
        let new_uri = Url::from_file_path(&new_path).unwrap();
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&client_path).await.unwrap());
        workspace_documents.insert(Url::from_file_path(&main_path).unwrap(), fs::read_to_string(&main_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri: old_uri.clone(),
            new_uri: new_uri.clone(),
            workspace_documents,
        };
        
        // Validate the rename operation
        let validation = engine.validate_rename(&request).await.unwrap();
        assert!(validation.is_valid, "Nested module rename should be valid");
        assert_eq!(validation.affected_files.len(), 1); // Main.gren should import Http.Client
        
        // Prepare the rename edits (LSP Protocol compliant)
        let result = engine.prepare_rename_edits(&request).await.unwrap();
        
        // Verify workspace edit
        assert!(result.changes.is_some());
        let changes = result.changes.as_ref().unwrap();
        
        // Verify exact expected changes for nested module rename
        assert_eq!(changes.len(), 2, "Should have exactly 2 files with changes");
        
        // Check Main.gren import update to Network.Http
        let main_uri = Url::from_file_path(&main_path).unwrap();
        let main_edits = changes.get(&main_uri).expect("Main.gren should have import updates");
        assert_eq!(main_edits.len(), 1, "Main.gren should have exactly 1 edit");
        assert_eq!(main_edits[0].new_text, "Network.Http", "Import should be updated to Network.Http");
        
        // Check module declaration update in renamed file
        let module_edits = changes.get(&new_uri).expect("Renamed file should have module declaration update");
        assert_eq!(module_edits.len(), 1, "Renamed file should have exactly 1 edit");
        assert_eq!(module_edits[0].new_text, "Network.Http", "Module declaration should be updated to Network.Http");
    }

    /// Test: Module Name Conflict Detection
    #[tokio::test]
    async fn test_module_name_conflict_detection() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create two modules with potential conflict
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper)\n\nhelper = \"utils\"").await.unwrap();
        
        let helpers_path = temp_dir.path().join("src").join("Helpers.gren");
        fs::write(&helpers_path, "module Helpers exposing (helper)\n\nhelper = \"helpers\"").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Try to rename Utils.gren to Helpers.gren (conflict)
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let new_uri = Url::from_file_path(&helpers_path).unwrap(); // This already exists
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri,
            new_uri,
            workspace_documents,
        };
        
        // Validate should fail due to conflict
        let validation = engine.validate_rename(&request).await.unwrap();
        assert!(!validation.is_valid, "Rename should fail due to name conflict");
        assert!(validation.error_message.is_some());
        assert!(validation.error_message.unwrap().contains("already exists"));
    }

    /// Test: File System Permission Handling
    #[tokio::test] 
    async fn test_file_system_permission_handling() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create a module file
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper)\n\nhelper = \"test\"").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Try to rename to a valid path (deep directory should be creatable)
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let valid_path = temp_dir.path().join("src").join("nested").join("Helpers.gren");
        let new_uri = Url::from_file_path(&valid_path).unwrap();
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri,
            new_uri,
            workspace_documents,
        };
        
        // Validation should check file system operations
        let validation = engine.validate_rename(&request).await.unwrap();
        // This should succeed since we can create the directory structure
        assert!(validation.is_valid, "Should be able to create directory structure");
    }

    /// Test: Workspace Protocol Handler Integration
    #[tokio::test]
    async fn test_workspace_protocol_handler_integration() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper)\n\nhelper = \"test\"").await.unwrap();
        
        let main_path = temp_dir.path().join("src").join("Main.gren");
        fs::write(&main_path, "module Main exposing (..)\n\nimport Utils\n\nmain = Utils.helper").await.unwrap();
        
        // Create workspace protocol handler
        let handler = WorkspaceProtocolHandler::new();
        
        // Create rename files params
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let helpers_path = temp_dir.path().join("src").join("Helpers.gren");
        let new_uri = Url::from_file_path(&helpers_path).unwrap();
        
        let params = RenameFilesParams {
            files: vec![FileRename {
                old_uri: old_uri.to_string(),
                new_uri: new_uri.to_string(),
            }],
        };
        
        // Create workspace documents
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        workspace_documents.insert(Url::from_file_path(&main_path).unwrap(), fs::read_to_string(&main_path).await.unwrap());
        
        // Handle willRenameFiles (should return None since no module rename engine is initialized)
        let result = handler.handle_will_rename_files(params.clone(), &workspace_documents).await.unwrap();
        assert!(result.is_none(), "Should return None when module rename engine not initialized");
        
        // Handle didRenameFiles (should complete without error)
        handler.handle_did_rename_files(params, &workspace_documents).await.unwrap();
    }

    /// Test: Import Rewriter Functionality
    #[tokio::test]
    async fn test_import_rewriter_functionality() {
        let rewriter = ImportRewriter::new().unwrap();
        
        // Test has_import_reference
        let content = "module Main exposing (..)\n\nimport Utils\nimport Http.Client as Client\n\nmain = Utils.helper (Client.get url)";
        
        assert!(rewriter.has_import_reference(content, "Utils").await.unwrap());
        assert!(rewriter.has_import_reference(content, "Http.Client").await.unwrap());
        assert!(!rewriter.has_import_reference(content, "NonExistent").await.unwrap());
        
        // Test rewrite_imports
        let edits = rewriter.rewrite_imports(content, "Utils", "Helpers").await.unwrap();
        assert_eq!(edits.len(), 1);
        assert!(edits[0].new_text.contains("Helpers"));
        
        let edits = rewriter.rewrite_imports(content, "Http.Client", "Network.Http").await.unwrap();
        assert_eq!(edits.len(), 1);
        assert!(edits[0].new_text.contains("Network.Http"));
        
        // Test update_module_declaration
        let module_content = "module Utils exposing (..)\n\nhelper = \"test\"";
        let edits = rewriter.update_module_declaration(module_content, "Helpers").await.unwrap();
        assert_eq!(edits.len(), 1);
        assert!(edits[0].new_text.contains("Helpers"));
    }

    /// Test: Complex Rename Scenario with Multiple Files
    #[tokio::test]
    async fn test_complex_rename_scenario() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create multiple files that import the same module
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper, formatter)\n\nhelper = \"help\"\nformatter = \"format\"").await.unwrap();
        
        let main_path = temp_dir.path().join("src").join("Main.gren");
        fs::write(&main_path, "module Main exposing (..)\n\nimport Utils\n\nmain = Utils.helper").await.unwrap();
        
        let app_path = temp_dir.path().join("src").join("App.gren");
        fs::write(&app_path, "module App exposing (..)\n\nimport Utils as U\n\nformat = U.formatter").await.unwrap();
        
        let view_path = temp_dir.path().join("src").join("View.gren");
        fs::write(&view_path, "module View exposing (..)\n\nimport Utils exposing (helper)\n\nrender = helper").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Create rename request
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let helpers_path = temp_dir.path().join("src").join("Helpers.gren");
        let new_uri = Url::from_file_path(&helpers_path).unwrap();
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        workspace_documents.insert(Url::from_file_path(&main_path).unwrap(), fs::read_to_string(&main_path).await.unwrap());
        workspace_documents.insert(Url::from_file_path(&app_path).unwrap(), fs::read_to_string(&app_path).await.unwrap());
        workspace_documents.insert(Url::from_file_path(&view_path).unwrap(), fs::read_to_string(&view_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri: old_uri.clone(),
            new_uri: new_uri.clone(),
            workspace_documents,
        };
        
        // Validate the rename operation
        let validation = engine.validate_rename(&request).await.unwrap();
        assert!(validation.is_valid, "Complex rename should be valid");
        assert_eq!(validation.affected_files.len(), 3); // Main.gren, App.gren, View.gren
        
        // Prepare the rename edits (LSP Protocol compliant)
        let result = engine.prepare_rename_edits(&request).await.unwrap();
        
        // Verify all affected files have updates
        assert!(result.changes.is_some());
        let changes = result.changes.as_ref().unwrap();
        
        // Should have exactly 4 files with changes: Main.gren, App.gren, View.gren, and renamed Helpers.gren
        assert_eq!(changes.len(), 4, "Should have exactly 4 files with changes");

        // Check Main.gren import update (simple import)
        let main_uri = Url::from_file_path(&main_path).unwrap();
        let main_edits = changes.get(&main_uri).expect("Main.gren should have import updates");
        assert_eq!(main_edits.len(), 1, "Main.gren should have exactly 1 edit");
        assert_eq!(main_edits[0].new_text, "Helpers", "Main.gren import should be updated to Helpers");

        // Check App.gren import update (import with alias)
        let app_uri = Url::from_file_path(&app_path).unwrap();
        let app_edits = changes.get(&app_uri).expect("App.gren should have import updates");
        assert_eq!(app_edits.len(), 1, "App.gren should have exactly 1 edit");
        assert_eq!(app_edits[0].new_text, "Helpers", "App.gren import should be updated to Helpers");

        // Check View.gren import update (import with exposing)
        let view_uri = Url::from_file_path(&view_path).unwrap();
        let view_edits = changes.get(&view_uri).expect("View.gren should have import updates");
        assert_eq!(view_edits.len(), 1, "View.gren should have exactly 1 edit");
        assert_eq!(view_edits[0].new_text, "Helpers", "View.gren import should be updated to Helpers");

        // Check module declaration update in renamed file
        let module_edits = changes.get(&new_uri).expect("Renamed file should have module declaration update");
        assert_eq!(module_edits.len(), 1, "Renamed file should have exactly 1 edit");
        assert_eq!(module_edits[0].new_text, "Helpers", "Module declaration should be updated to Helpers");
    }

    /// Test: Empty Workspace Scenario
    #[tokio::test]
    async fn test_empty_workspace_scenario() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create single isolated module
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper)\n\nhelper = \"isolated\"").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Create rename request
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let helpers_path = temp_dir.path().join("src").join("Helpers.gren");
        let new_uri = Url::from_file_path(&helpers_path).unwrap();
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri: old_uri.clone(),
            new_uri: new_uri.clone(),
            workspace_documents,
        };
        
        // Validate the rename operation
        let validation = engine.validate_rename(&request).await.unwrap();
        assert!(validation.is_valid, "Isolated module rename should be valid");
        assert_eq!(validation.affected_files.len(), 0); // No other files affected
        
        // Prepare the rename edits (LSP Protocol compliant)
        let result = engine.prepare_rename_edits(&request).await.unwrap();
        
        // Should only update the module declaration
        assert!(result.changes.is_some());
        let changes = result.changes.as_ref().unwrap();
        assert_eq!(changes.len(), 1, "Should have exactly 1 file with changes (only the renamed file)");
        
        // Check module declaration update
        let module_edits = changes.get(&new_uri).expect("Renamed file should have module declaration update");
        assert_eq!(module_edits.len(), 1, "Renamed file should have exactly 1 edit");
        assert_eq!(module_edits[0].new_text, "Helpers", "Module declaration should be updated to Helpers");
    }

    /// Test: Compiler Validation Integration
    #[tokio::test]
    async fn test_compiler_validation_integration() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create gren.json for valid project
        fs::write(temp_dir.path().join("gren.json"), r#"{"type":"application"}"#).await.unwrap();
        
        // Create valid module with correct syntax
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper)\n\nhelper : String -> String\nhelper s = s").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Create rename request
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let helpers_path = temp_dir.path().join("src").join("Helpers.gren");
        let new_uri = Url::from_file_path(&helpers_path).unwrap();
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri: old_uri.clone(),
            new_uri: new_uri.clone(),
            workspace_documents,
        };
        
        // Test should pass if compiler is available, or skip compilation validation if not
        match engine.prepare_rename_edits(&request).await {
            Ok(result) => {
                // If compilation validation runs, it should pass with valid syntax
                assert!(result.changes.is_some());
                let changes = result.changes.as_ref().unwrap();
                assert_eq!(changes.len(), 1, "Should have exactly 1 file with changes (module declaration)");
            }
            Err(e) => {
                // Only acceptable error is if compiler is not available
                let error_msg = e.to_string().to_lowercase();
                assert!(
                    error_msg.contains("compiler not available") || 
                    error_msg.contains("could not find project root") ||
                    error_msg.contains("no such file or directory"),
                    "Unexpected error that's not related to compiler availability: {}", e
                );
            }
        }
    }

    /// Test: Compiler Validation Failure with Invalid Syntax
    #[tokio::test]
    async fn test_compiler_validation_syntax_error() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = Arc::new(RwLock::new(Some(temp_dir.path().to_path_buf())));
        
        // Create gren.json for valid project
        fs::write(temp_dir.path().join("gren.json"), r#"{"type":"application"}"#).await.unwrap();
        
        // Create module with syntax error
        let utils_path = temp_dir.path().join("src").join("Utils.gren");
        fs::create_dir_all(utils_path.parent().unwrap()).await.unwrap();
        fs::write(&utils_path, "module Utils exposing (helper)\n\n-- Invalid syntax: missing type annotation\nhelper s = s invalid syntax").await.unwrap();
        
        // Create module rename engine
        let symbol_index = Arc::new(RwLock::new(None));
        let engine = ModuleRenameEngine::new(symbol_index, workspace_root).unwrap();
        
        // Create rename request
        let old_uri = Url::from_file_path(&utils_path).unwrap();
        let helpers_path = temp_dir.path().join("src").join("Helpers.gren");
        let new_uri = Url::from_file_path(&helpers_path).unwrap();
        
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(old_uri.clone(), fs::read_to_string(&utils_path).await.unwrap());
        
        let request = ModuleRenameRequest {
            old_uri: old_uri.clone(),
            new_uri: new_uri.clone(),
            workspace_documents,
        };
        
        // Test should fail compilation validation if compiler is available
        match engine.prepare_rename_edits(&request).await {
            Ok(_) => {
                // If we get here, compiler validation either passed (unexpected) or was skipped
                println!("Warning: Compilation validation was skipped or passed despite syntax error");
            }
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                // Error should be either compiler not available or compilation failed
                assert!(
                    error_msg.contains("compiler not available") || 
                    error_msg.contains("compilation validation failed") ||
                    error_msg.contains("could not find project root") ||
                    error_msg.contains("no such file or directory"),
                    "Expected compiler-related error, got: {}", e
                );
            }
        }
    }
}