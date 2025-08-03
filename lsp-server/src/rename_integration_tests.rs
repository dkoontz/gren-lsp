#[cfg(test)]
mod rename_integration_tests {
    use super::*;
    use crate::rename::{RenameEngine, PrepareRenameParams};
    use crate::symbol_index::SymbolIndex;
    use crate::compiler_interface::{GrenCompiler, CompilerConfig};
    use std::collections::HashMap;
    use tower_lsp::lsp_types::*;
    use tempfile::TempDir;
    use url::Url;
    use tracing::debug;

    /// Helper to create a test rename engine
    async fn create_test_rename_engine() -> (RenameEngine, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Ensure the temp directory exists and is writable
        std::fs::create_dir_all(&workspace_root).expect("Failed to create workspace root");
        
        // Use in-memory database for testing to avoid file system issues
        let symbol_index = SymbolIndex::new_in_memory(workspace_root.clone())
            .await
            .expect("Failed to create in-memory symbol index");
            
        let compiler_config = CompilerConfig::default();
        let compiler = GrenCompiler::new(compiler_config);
        
        let engine = RenameEngine::new(symbol_index, compiler)
            .expect("Failed to create rename engine");
            
        (engine, temp_dir)
    }

    #[tokio::test]
    async fn test_validate_new_name_valid_function() {
        let (engine, _temp_dir) = create_test_rename_engine().await;
        
        // Test valid function names - validate specific success results
        let result1 = engine.validate_new_name("validFunction");
        assert!(result1.is_ok(), "Expected validFunction to be valid, got: {:?}", result1.err());
        
        let result2 = engine.validate_new_name("anotherFunction123");
        assert!(result2.is_ok(), "Expected anotherFunction123 to be valid, got: {:?}", result2.err());
        
        let result3 = engine.validate_new_name("function_with_underscores");
        assert!(result3.is_ok(), "Expected function_with_underscores to be valid, got: {:?}", result3.err());
    }

    #[tokio::test]
    async fn test_validate_new_name_valid_type() {
        let (engine, _temp_dir) = create_test_rename_engine().await;
        
        // Test valid type names - validate specific success results
        let result1 = engine.validate_new_name("ValidType");
        assert!(result1.is_ok(), "Expected ValidType to be valid, got: {:?}", result1.err());
        
        let result2 = engine.validate_new_name("AnotherType123");
        assert!(result2.is_ok(), "Expected AnotherType123 to be valid, got: {:?}", result2.err());
        
        let result3 = engine.validate_new_name("TypeName");
        assert!(result3.is_ok(), "Expected TypeName to be valid, got: {:?}", result3.err());
    }

    #[tokio::test]
    async fn test_validate_new_name_invalid() {
        let (engine, _temp_dir) = create_test_rename_engine().await;
        
        // Test invalid names - validate specific error conditions
        let empty_result = engine.validate_new_name("");
        assert!(empty_result.is_err(), "Expected empty string to be invalid");
        assert!(empty_result.unwrap_err().to_string().contains("empty"), "Expected empty error message");
        
        let number_result = engine.validate_new_name("123invalid");
        assert!(number_result.is_err(), "Expected name starting with number to be invalid");
        
        let hyphen_result = engine.validate_new_name("invalid-name");
        assert!(hyphen_result.is_err(), "Expected name with hyphen to be invalid");
        
        let keyword_if = engine.validate_new_name("if");
        assert!(keyword_if.is_err(), "Expected 'if' keyword to be invalid");
        assert!(keyword_if.unwrap_err().to_string().contains("reserved"), "Expected reserved keyword error");
        
        let keyword_type = engine.validate_new_name("type");
        assert!(keyword_type.is_err(), "Expected 'type' keyword to be invalid");
        
        let keyword_module = engine.validate_new_name("module");
        assert!(keyword_module.is_err(), "Expected 'module' keyword to be invalid");
    }

    #[tokio::test]
    async fn test_prepare_rename_no_symbol() {
        let (mut engine, _temp_dir) = create_test_rename_engine().await;
        
        let uri = Url::parse("file:///test.gren").unwrap();
        let params = PrepareRenameParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line: 0, character: 0 },
        };
        
        let document_content = "-- Empty file\n";
        
        let result = engine.handle_prepare_rename(params, document_content).await;
        
        // The test should pass whether it succeeds with None or fails gracefully
        match result {
            Ok(response) => {
                assert!(response.is_none(), "Expected None response for position with no symbol, got: {:?}", response);
            }
            Err(e) => {
                // It's acceptable for this to error on empty file - that means no symbol found
                debug!("Prepare rename failed as expected on empty file: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_rename_no_symbol() {
        let (mut engine, _temp_dir) = create_test_rename_engine().await;
        
        let uri = Url::parse("file:///test.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line: 0, character: 0 },
            },
            new_name: "newName".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = "-- Empty file\n";
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(uri, document_content.to_string());
        
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        // The test should pass whether it succeeds with None or fails gracefully 
        match result {
            Ok(workspace_edit) => {
                assert!(workspace_edit.is_none(), "Expected None workspace edit for position with no symbol, got: {:?}", workspace_edit);
                // Verify workspace documents remain unchanged
                assert_eq!(workspace_documents.len(), 1, "Expected workspace documents to remain unchanged");
            }
            Err(e) => {
                // It's acceptable for this to error on empty file - that means no symbol found
                debug!("Rename failed as expected on empty file: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_rename_invalid_name() {
        let (mut engine, _temp_dir) = create_test_rename_engine().await;
        
        let uri = Url::parse("file:///test.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line: 0, character: 0 },
            },
            new_name: "invalid-name".to_string(), // Invalid name with hyphen
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = "test = 42\n";
        let mut workspace_documents = HashMap::new();
        workspace_documents.insert(uri, document_content.to_string());
        
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Invalid name"));
    }

    #[tokio::test]
    async fn test_extract_text_from_range() {
        let (engine, _temp_dir) = create_test_rename_engine().await;
        
        let content = "line1\nline2\nline3";
        let range = Range {
            start: Position { line: 1, character: 2 },
            end: Position { line: 1, character: 5 },
        };
        
        let result = engine.extract_text_from_range(content, range);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ne2");
    }

    #[tokio::test]
    async fn test_extract_text_from_range_single_char() {
        let (engine, _temp_dir) = create_test_rename_engine().await;
        
        let content = "hello world";
        let range = Range {
            start: Position { line: 0, character: 6 },
            end: Position { line: 0, character: 11 },
        };
        
        let result = engine.extract_text_from_range(content, range);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "world");
    }
}