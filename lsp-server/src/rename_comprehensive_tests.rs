#[cfg(test)]
mod rename_comprehensive_tests {
    use super::*;
    use crate::rename::{RenameEngine, PrepareRenameParams};
    use crate::symbol_index::SymbolIndex;
    use crate::compiler_interface::{GrenCompiler, CompilerConfig};
    use std::collections::HashMap;
    use tower_lsp::lsp_types::*;
    use tempfile::TempDir;
    use url::Url;
    use tracing::debug;
    use std::fs;

    /// Helper to create a test rename engine with sample project
    async fn create_test_project_with_rename_engine() -> (RenameEngine, TempDir, HashMap<Url, String>) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Create sample project structure
        let src_dir = workspace_root.join("src");
        fs::create_dir_all(&src_dir).expect("Failed to create src directory");
        
        // Create gren.json
        let gren_json = r#"{
    "type": "application",
    "source-directories": ["src"],
    "gren-version": "0.6.0"
}"#;
        fs::write(workspace_root.join("gren.json"), gren_json).expect("Failed to write gren.json");
        
        // Create sample Gren files for testing
        let mut workspace_documents = HashMap::new();
        
        // Main.gren - Entry point with function calls
        let main_content = r#"module Main exposing (main)

import User exposing (createUser, User)
import Utils.Helper exposing (formatName)

main : String
main =
    let
        user = createUser "Alice"
        formattedName = formatName user.name
    in
    "Hello " ++ formattedName
"#;
        let main_uri = Url::parse("file:///test/src/Main.gren").unwrap();
        workspace_documents.insert(main_uri.clone(), main_content.to_string());
        fs::write(src_dir.join("Main.gren"), main_content).expect("Failed to write Main.gren");
        
        // User.gren - Module with types and functions
        let user_content = r#"module User exposing (User, createUser, validateUser)

type User = User
    { name : String
    , email : String
    }

createUser : String -> User
createUser name =
    User { name = name, email = "" }

validateUser : User -> Bool
validateUser user =
    when user is
        User record -> String.length record.name > 0
"#;
        let user_uri = Url::parse("file:///test/src/User.gren").unwrap();
        workspace_documents.insert(user_uri.clone(), user_content.to_string());
        fs::write(src_dir.join("User.gren"), user_content).expect("Failed to write User.gren");
        
        // Utils/Helper.gren - Nested module
        let utils_dir = src_dir.join("Utils");
        fs::create_dir_all(&utils_dir).expect("Failed to create Utils directory");
        let helper_content = r#"module Utils.Helper exposing (formatName, capitalizeFirst)

formatName : String -> String
formatName name =
    capitalizeFirst name

capitalizeFirst : String -> String
capitalizeFirst str =
    if String.length str > 0 then
        String.toUpper (String.left 1 str) ++ String.toLower (String.dropLeft 1 str)
    else
        str
"#;
        let helper_uri = Url::parse("file:///test/src/Utils/Helper.gren").unwrap();
        workspace_documents.insert(helper_uri.clone(), helper_content.to_string());
        fs::write(utils_dir.join("Helper.gren"), helper_content).expect("Failed to write Helper.gren");
        
        // Create in-memory symbol index for testing
        let symbol_index = SymbolIndex::new_in_memory(workspace_root.clone())
            .await
            .expect("Failed to create in-memory symbol index");
            
        // Populate symbol index with symbols from our test files
        use crate::symbol_index::Symbol;
        use tower_lsp::lsp_types::SymbolKind;
        
        // Add symbols from Main.gren
        let main_symbols = vec![
            Symbol::new(
                "main".to_string(),
                SymbolKind::FUNCTION,
                &main_uri,
                Range {
                    start: Position { line: 5, character: 0 },
                    end: Position { line: 5, character: 4 },
                },
                Some("Main".to_string()),
                Some("String".to_string()),
                None,
            ),
        ];
        
        // Add symbols from User.gren
        let user_symbols = vec![
            Symbol::new(
                "User".to_string(),
                SymbolKind::ENUM, // Gren custom types are like enums
                &user_uri,
                Range {
                    start: Position { line: 2, character: 5 },
                    end: Position { line: 2, character: 9 },
                },
                Some("User".to_string()),
                None,
                None,
            ),
            Symbol::new(
                "createUser".to_string(),
                SymbolKind::FUNCTION,
                &user_uri,
                Range {
                    start: Position { line: 6, character: 0 },
                    end: Position { line: 6, character: 10 },
                },
                Some("User".to_string()),
                Some("String -> User".to_string()),
                None,
            ),
            Symbol::new(
                "validateUser".to_string(),
                SymbolKind::FUNCTION,
                &user_uri,
                Range {
                    start: Position { line: 10, character: 0 },
                    end: Position { line: 10, character: 12 },
                },
                Some("User".to_string()),
                Some("User -> Bool".to_string()),
                None,
            ),
        ];
        
        // Add symbols from Utils/Helper.gren
        let helper_symbols = vec![
            Symbol::new(
                "formatName".to_string(),
                SymbolKind::FUNCTION,
                &helper_uri,
                Range {
                    start: Position { line: 2, character: 0 },
                    end: Position { line: 2, character: 10 },
                },
                Some("Utils.Helper".to_string()),
                Some("String -> String".to_string()),
                None,
            ),
            Symbol::new(
                "capitalizeFirst".to_string(),
                SymbolKind::FUNCTION,
                &helper_uri,
                Range {
                    start: Position { line: 5, character: 0 },
                    end: Position { line: 5, character: 15 },
                },
                Some("Utils.Helper".to_string()),
                Some("String -> String".to_string()),
                None,
            ),
        ];
        
        // Add all symbols to index
        for symbol in main_symbols.iter().chain(user_symbols.iter()).chain(helper_symbols.iter()) {
            symbol_index.add_symbol(symbol).await.expect("Failed to add symbol to index");
        }
        
        // Add references to the index for cross-module testing
        use crate::symbol_index::{SymbolReference, ReferenceKind};
        
        let references = vec![
            // createUser function definition
            SymbolReference::new(
                "createUser".to_string(),
                &user_uri,
                Range {
                    start: Position { line: 6, character: 0 },
                    end: Position { line: 6, character: 10 },
                },
                ReferenceKind::Usage,
            ),
            // createUser usage in Main.gren
            SymbolReference::new(
                "createUser".to_string(),
                &main_uri,
                Range {
                    start: Position { line: 8, character: 17 },
                    end: Position { line: 8, character: 27 },
                },
                ReferenceKind::Usage,
            ),
            // formatName function definition
            SymbolReference::new(
                "formatName".to_string(),
                &helper_uri,
                Range {
                    start: Position { line: 2, character: 0 },
                    end: Position { line: 2, character: 10 },
                },
                ReferenceKind::Usage,
            ),
            // formatName usage within same file (function definition)
            SymbolReference::new(
                "formatName".to_string(),
                &helper_uri,
                Range {
                    start: Position { line: 3, character: 0 },
                    end: Position { line: 3, character: 10 },
                },
                ReferenceKind::Usage,
            ),
            // User type definition
            SymbolReference::new(
                "User".to_string(),
                &user_uri,
                Range {
                    start: Position { line: 2, character: 5 },
                    end: Position { line: 2, character: 9 },
                },
                ReferenceKind::Usage,
            ),
            // User type usage in Main.gren import
            SymbolReference::new(
                "User".to_string(),
                &main_uri,
                Range {
                    start: Position { line: 2, character: 25 },
                    end: Position { line: 2, character: 29 },
                },
                ReferenceKind::Usage,
            ),
        ];
        
        symbol_index.add_references(&references).await.expect("Failed to add references to index");
            
        let compiler_config = CompilerConfig::default();
        let compiler = GrenCompiler::new(compiler_config);
        
        let engine = RenameEngine::new(symbol_index, compiler)
            .expect("Failed to create rename engine");
            
        (engine, temp_dir, workspace_documents)
    }

    #[tokio::test]
    async fn test_local_symbol_rename_function() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Test renaming formatName function in Utils.Helper
        let helper_uri = Url::parse("file:///test/src/Utils/Helper.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: helper_uri.clone() },
                position: Position { line: 2, character: 0 }, // formatName function
            },
            new_name: "processName".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = workspace_documents.get(&helper_uri).unwrap();
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        // Validate the rename operation
        assert!(result.is_ok(), "Expected rename to succeed, got error: {:?}", result.err());
        
        let workspace_edit = result.unwrap();
        assert!(workspace_edit.is_some(), "Expected WorkspaceEdit for valid symbol rename");
        
        let edit = workspace_edit.unwrap();
        assert!(edit.changes.is_some(), "Expected changes in WorkspaceEdit");
        
        let changes = edit.changes.unwrap();
        
        // Should update both the definition and the call within the same file
        assert!(changes.contains_key(&helper_uri), "Expected changes to Helper.gren file");
        let helper_changes = &changes[&helper_uri];
        assert!(helper_changes.len() >= 2, "Expected at least 2 changes: definition and usage in formatName function");
        
        // Validate that all changes replace 'formatName' with 'processName'
        for text_edit in helper_changes {
            assert_eq!(text_edit.new_text, "processName", 
                "Expected all text edits to replace with 'processName', got: '{}'", text_edit.new_text);
        }
    }

    #[tokio::test]
    async fn test_cross_module_function_rename() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Test renaming createUser function - should update User.gren and Main.gren
        let user_uri = Url::parse("file:///test/src/User.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: user_uri.clone() },
                position: Position { line: 8, character: 0 }, // createUser function definition
            },
            new_name: "makeUser".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = workspace_documents.get(&user_uri).unwrap();
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        // Validate the rename operation
        assert!(result.is_ok(), "Expected cross-module rename to succeed, got error: {:?}", result.err());
        
        let workspace_edit = result.unwrap();
        assert!(workspace_edit.is_some(), "Expected WorkspaceEdit for cross-module rename");
        
        let edit = workspace_edit.unwrap();
        assert!(edit.changes.is_some(), "Expected changes in WorkspaceEdit");
        
        let changes = edit.changes.unwrap();
        
        // Should update both User.gren (definition) and Main.gren (import and usage)
        assert!(changes.contains_key(&user_uri), "Expected changes to User.gren file");
        
        let main_uri = Url::parse("file:///test/src/Main.gren").unwrap();
        assert!(changes.contains_key(&main_uri), "Expected changes to Main.gren file for cross-module rename");
        
        // Validate User.gren changes (definition and type annotation)
        let user_changes = &changes[&user_uri];
        assert!(user_changes.len() >= 1, "Expected at least 1 change in User.gren (function definition)");
        
        // Validate Main.gren changes (import and usage)
        let main_changes = &changes[&main_uri];
        assert!(main_changes.len() >= 2, "Expected at least 2 changes in Main.gren (import and usage)");
        
        // Validate all changes replace 'createUser' with 'makeUser'
        for (uri, text_edits) in &changes {
            for text_edit in text_edits {
                assert_eq!(text_edit.new_text, "makeUser", 
                    "Expected all text edits in {} to replace with 'makeUser', got: '{}'", 
                    uri, text_edit.new_text);
            }
        }
    }

    #[tokio::test]
    async fn test_type_rename_with_constructors() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Test renaming User type - should update type definition, constructor, and pattern matches
        let user_uri = Url::parse("file:///test/src/User.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: user_uri.clone() },
                position: Position { line: 2, character: 5 }, // User type definition
            },
            new_name: "Person".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = workspace_documents.get(&user_uri).unwrap();
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        // Validate the rename operation
        assert!(result.is_ok(), "Expected type rename to succeed, got error: {:?}", result.err());
        
        let workspace_edit = result.unwrap();
        assert!(workspace_edit.is_some(), "Expected WorkspaceEdit for type rename");
        
        let edit = workspace_edit.unwrap();
        assert!(edit.changes.is_some(), "Expected changes in WorkspaceEdit");
        
        let changes = edit.changes.unwrap();
        
        // Should update User.gren (type definition, constructor, type annotations, pattern matches)
        assert!(changes.contains_key(&user_uri), "Expected changes to User.gren file");
        
        let main_uri = Url::parse("file:///test/src/Main.gren").unwrap();
        assert!(changes.contains_key(&main_uri), "Expected changes to Main.gren file for cross-module type usage");
        
        // Validate User.gren changes include type definition, constructor, and pattern match
        let user_changes = &changes[&user_uri];
        assert!(user_changes.len() >= 3, 
            "Expected at least 3 changes in User.gren (type def, constructor, pattern match), got: {}", 
            user_changes.len());
        
        // Validate all changes replace 'User' with 'Person'
        for text_edit in user_changes {
            assert_eq!(text_edit.new_text, "Person", 
                "Expected all text edits to replace with 'Person', got: '{}'", text_edit.new_text);
        }
    }

    #[tokio::test]
    async fn test_rename_validation_existing_name_conflict() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Try to rename createUser to validateUser (which already exists)
        let user_uri = Url::parse("file:///test/src/User.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: user_uri.clone() },
                position: Position { line: 6, character: 0 }, // createUser function
            },
            new_name: "validateUser".to_string(), // Conflicts with existing function
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = workspace_documents.get(&user_uri).unwrap();
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        // Should fail due to naming conflict
        assert!(result.is_err(), "Expected rename to fail due to name conflict");
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.to_lowercase().contains("already exists") || 
                error_message.to_lowercase().contains("conflict"),
            "Expected error message to mention conflict, got: '{}'", error_message);
    }

    #[tokio::test]
    async fn test_rename_validation_reserved_keywords() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Try to rename to reserved keywords
        let helper_uri = Url::parse("file:///test/src/Utils/Helper.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: helper_uri.clone() },
                position: Position { line: 2, character: 0 }, // formatName function
            },
            new_name: "type".to_string(), // Reserved keyword
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = workspace_documents.get(&helper_uri).unwrap();
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        // Should fail due to reserved keyword
        assert!(result.is_err(), "Expected rename to fail for reserved keyword");
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.to_lowercase().contains("reserved") || 
                error_message.to_lowercase().contains("keyword"),
            "Expected error message to mention reserved keyword, got: '{}'", error_message);
    }

    #[tokio::test]
    async fn test_rename_validation_invalid_naming_conventions() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Try to rename function to invalid name (starts with uppercase)
        let helper_uri = Url::parse("file:///test/src/Utils/Helper.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: helper_uri.clone() },
                position: Position { line: 2, character: 0 }, // formatName function
            },
            new_name: "FormatName".to_string(), // Invalid: function names should start lowercase
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = workspace_documents.get(&helper_uri).unwrap();
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        // Should fail due to naming convention violation
        assert!(result.is_err(), "Expected rename to fail for invalid naming convention");
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.to_lowercase().contains("lowercase") || 
                error_message.to_lowercase().contains("naming") ||
                error_message.to_lowercase().contains("convention"),
            "Expected error message to mention naming convention, got: '{}'", error_message);
    }

    #[tokio::test]
    async fn test_prepare_rename_valid_symbol() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Test prepare rename on valid symbol
        let user_uri = Url::parse("file:///test/src/User.gren").unwrap();
        let params = PrepareRenameParams {
            text_document: TextDocumentIdentifier { uri: user_uri.clone() },
            position: Position { line: 6, character: 0 }, // createUser function
        };
        
        let document_content = workspace_documents.get(&user_uri).unwrap();
        let result = engine.handle_prepare_rename(params, document_content).await;
        
        // Should succeed and return range information
        assert!(result.is_ok(), "Expected prepare rename to succeed, got error: {:?}", result.err());
        
        let response = result.unwrap();
        assert!(response.is_some(), "Expected PrepareRenameResponse for valid symbol");
        
        match response.unwrap() {
            PrepareRenameResponse::Range(range) => {
                assert_eq!(range.start.line, 6, "Expected prepare rename range to start at line 6");
                assert!(range.start.character <= 11, "Expected range to include 'createUser' function name");
            },
            PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } => {
                assert_eq!(range.start.line, 6, "Expected prepare rename range to start at line 6");
                assert_eq!(placeholder, "createUser", "Expected placeholder to be 'createUser'");
            },
            PrepareRenameResponse::DefaultBehavior { default_behavior: _ } => {
                // This is also acceptable
            }
        }
    }

    #[tokio::test] 
    async fn test_rename_workspace_edit_structure() {
        let (mut engine, _temp_dir, workspace_documents) = create_test_project_with_rename_engine().await;
        
        // Test that WorkspaceEdit structure follows LSP 3.18 spec
        let helper_uri = Url::parse("file:///test/src/Utils/Helper.gren").unwrap();
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: helper_uri.clone() },
                position: Position { line: 2, character: 0 }, // formatName function
            },
            new_name: "processName".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let document_content = workspace_documents.get(&helper_uri).unwrap();
        let result = engine.handle_rename(params, document_content, &workspace_documents).await;
        
        assert!(result.is_ok(), "Expected rename to succeed for LSP structure test");
        
        let workspace_edit = result.unwrap();
        assert!(workspace_edit.is_some(), "Expected WorkspaceEdit");
        
        let edit = workspace_edit.unwrap();
        
        // Validate WorkspaceEdit structure
        assert!(edit.changes.is_some(), "Expected 'changes' field in WorkspaceEdit");
        assert!(edit.document_changes.is_none(), "Expected 'document_changes' to be None for this test");
        assert!(edit.change_annotations.is_none(), "Expected 'change_annotations' to be None for this test");
        
        let changes = edit.changes.unwrap();
        
        // Validate TextEdit structure for each file
        for (uri, text_edits) in &changes {
            assert!(!text_edits.is_empty(), "Expected at least one TextEdit for {}", uri);
            
            for text_edit in text_edits {
                // Validate Range structure
                assert!(text_edit.range.start.line <= text_edit.range.end.line, 
                    "Expected start line <= end line in TextEdit range");
                    
                if text_edit.range.start.line == text_edit.range.end.line {
                    assert!(text_edit.range.start.character <= text_edit.range.end.character,
                        "Expected start character <= end character for single-line range");
                }
                
                // Validate new_text is not empty and meaningful
                assert!(!text_edit.new_text.trim().is_empty(), 
                    "Expected non-empty new_text in TextEdit");
                assert_eq!(text_edit.new_text, "processName", 
                    "Expected new_text to be 'processName'");
            }
        }
        
        // Validate that ranges don't overlap within each file
        for (uri, text_edits) in &changes {
            for (i, edit1) in text_edits.iter().enumerate() {
                for (j, edit2) in text_edits.iter().enumerate() {
                    if i != j {
                        // Check for range overlap
                        let overlap = !(edit1.range.end.line < edit2.range.start.line ||
                                      (edit1.range.end.line == edit2.range.start.line && 
                                       edit1.range.end.character <= edit2.range.start.character) ||
                                      edit2.range.end.line < edit1.range.start.line ||
                                      (edit2.range.end.line == edit1.range.start.line && 
                                       edit2.range.end.character <= edit1.range.start.character));
                        
                        assert!(!overlap, 
                            "Found overlapping TextEdit ranges in {}: {:?} and {:?}", 
                            uri, edit1.range, edit2.range);
                    }
                }
            }
        }
    }
}