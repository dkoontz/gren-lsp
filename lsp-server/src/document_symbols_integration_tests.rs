#[cfg(test)]
mod document_symbols_integration_tests {
    use crate::document_symbols::DocumentSymbolsEngine;
    use crate::symbol_index::SymbolIndex;
    use anyhow::Result;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tower_lsp::lsp_types::*;

    async fn setup_test_index() -> Result<SymbolIndex> {
        let temp_dir = tempfile::tempdir()?;
        SymbolIndex::new_in_memory(temp_dir.path().to_path_buf()).await
    }

    async fn create_test_file_with_symbols() -> Result<(Url, SymbolIndex)> {
        let index = setup_test_index().await?;
        let uri = Url::from_file_path("/test/complex_module.gren").map_err(|_| anyhow::anyhow!("Invalid URI"))?;

        // Create a complex Gren file with various symbol types
        let content = r#"module TestModule exposing (..)

{-| A user type definition
-}
type User =
    User String Int

{-| Status enumeration
-}
type Status 
    = Active
    | Inactive
    | Pending

{-| Calculate user age
-}
calculateAge : User -> Int
calculateAge (User _ age) = 
    age

{-| Process a user with status
-}
processUser : User -> Status -> String
processUser user status =
    when status is
        Active -> "Processing active user"
        Inactive -> "User is inactive"
        Pending -> "User pending approval"

{-| Default user constant
-}
defaultUser : User
defaultUser = User "Anonymous" 0

{-| Configuration record type
-}
type alias Config =
    { timeout : Int
    , retries : Int
    }
"#;

        // Index the content with realistic symbols
        index.index_file(&uri, content).await?;
        
        Ok((uri, index))
    }

    #[tokio::test]
    async fn test_document_symbols_basic_workflow() -> Result<()> {
        let (uri, symbol_index) = create_test_file_with_symbols().await?;
        
        let engine = DocumentSymbolsEngine::new(Arc::new(RwLock::new(Some(symbol_index))));
        
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = engine.handle_document_symbol(params).await?;
        
        assert!(result.is_some(), "Should return document symbols");
        
        if let Some(DocumentSymbolResponse::Nested(symbols)) = result {
            // Verify we have symbols
            assert!(!symbols.is_empty(), "Should have at least one symbol");
            
            // Check for expected symbol types
            let symbol_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
            
            // Should contain module, types, and functions
            println!("Found symbols: {:?}", symbol_names);
            
            // Basic validation - we expect to find various symbol types
            let has_module = symbols.iter().any(|s| s.kind == SymbolKind::MODULE);
            let has_types = symbols.iter().any(|s| matches!(s.kind, SymbolKind::CLASS | SymbolKind::ENUM | SymbolKind::STRUCT));
            let has_functions = symbols.iter().any(|s| s.kind == SymbolKind::FUNCTION);
            
            assert!(has_module || has_types || has_functions, "Should have at least modules, types, or functions");
        } else {
            panic!("Expected nested document symbols");
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_symbols_hierarchy_structure() -> Result<()> {
        let (uri, symbol_index) = create_test_file_with_symbols().await?;
        
        let engine = DocumentSymbolsEngine::new(Arc::new(RwLock::new(Some(symbol_index))));
        
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = engine.handle_document_symbol(params).await?;
        
        if let Some(DocumentSymbolResponse::Nested(symbols)) = result {
            // Verify hierarchical structure
            for symbol in &symbols {
                // Check that ranges are valid
                assert!(symbol.range.start.line <= symbol.range.end.line, 
                    "Symbol range should be valid: {:?}", symbol.name);
                
                // Check that selection range is within the main range
                assert!(symbol.selection_range.start.line >= symbol.range.start.line &&
                       symbol.selection_range.end.line <= symbol.range.end.line,
                    "Selection range should be within symbol range for: {}", symbol.name);
                
                // If symbol has children, verify they are within parent range
                if let Some(children) = &symbol.children {
                    for child in children {
                        assert!(child.range.start.line >= symbol.range.start.line &&
                               child.range.end.line <= symbol.range.end.line,
                            "Child symbol {} should be within parent {} range", 
                            child.name, symbol.name);
                    }
                }
            }
            
            // Verify symbols are sorted by position
            for window in symbols.windows(2) {
                let first = &window[0];
                let second = &window[1];
                assert!(first.range.start.line <= second.range.start.line,
                    "Symbols should be sorted by line position: {} vs {}", 
                    first.name, second.name);
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_symbols_kinds() -> Result<()> {
        let (uri, symbol_index) = create_test_file_with_symbols().await?;
        
        let engine = DocumentSymbolsEngine::new(Arc::new(RwLock::new(Some(symbol_index))));
        
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = engine.handle_document_symbol(params).await?;
        
        if let Some(DocumentSymbolResponse::Nested(symbols)) = result {
            // Collect all symbols including children
            let mut all_symbols = Vec::new();
            for symbol in &symbols {
                all_symbols.push(symbol);
                if let Some(children) = &symbol.children {
                    all_symbols.extend(children);
                }
            }
            
            // Verify symbol kinds make sense
            for symbol in all_symbols {
                match symbol.kind {
                    SymbolKind::MODULE => {
                        assert!(symbol.name.ends_with("Module") || symbol.name == "TestModule",
                            "Module symbol should have module name: {}", symbol.name);
                    }
                    SymbolKind::CLASS | SymbolKind::ENUM | SymbolKind::STRUCT => {
                        // Type definitions (User, Status, Config, etc.)
                        assert!(symbol.name.chars().next().unwrap().is_uppercase(),
                            "Type symbol should start with uppercase: {}", symbol.name);
                    }
                    SymbolKind::FUNCTION => {
                        // Functions (calculateAge, processUser, etc.)
                        assert!(symbol.name.chars().next().unwrap().is_lowercase(),
                            "Function symbol should start with lowercase: {}", symbol.name);
                    }
                    SymbolKind::CONSTRUCTOR => {
                        // Type constructors (User, Active, Inactive, etc.)
                        assert!(symbol.name.chars().next().unwrap().is_uppercase(),
                            "Constructor symbol should start with uppercase: {}", symbol.name);
                    }
                    SymbolKind::CONSTANT | SymbolKind::VARIABLE => {
                        // Constants (defaultUser)
                        assert!(symbol.name.chars().next().unwrap().is_lowercase(),
                            "Constant symbol should start with lowercase: {}", symbol.name);
                    }
                    _ => {
                        // Other kinds are acceptable
                    }
                }
            }
        }
        
        Ok(())
    }

    #[tokio::test] 
    async fn test_document_symbols_empty_file() -> Result<()> {
        let index = setup_test_index().await?;
        let uri = Url::from_file_path("/test/empty.gren").map_err(|_| anyhow::anyhow!("Invalid URI"))?;
        
        // Index empty content
        index.index_file(&uri, "").await?;
        
        let engine = DocumentSymbolsEngine::new(Arc::new(RwLock::new(Some(index))));
        
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = engine.handle_document_symbol(params).await?;
        
        // Empty file should return None or empty symbols
        assert!(result.is_none() || matches!(result, Some(DocumentSymbolResponse::Nested(ref symbols)) if symbols.is_empty()),
            "Empty file should return no symbols");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_symbols_range_accuracy() -> Result<()> {
        let index = setup_test_index().await?;
        let uri = Url::from_file_path("/test/simple.gren").map_err(|_| anyhow::anyhow!("Invalid URI"))?;

        // Simple file with known line positions
        let content = r#"module Simple exposing (..)

type Status = Active | Inactive

getValue : Int
getValue = 42
"#;

        index.index_file(&uri, content).await?;
        
        let engine = DocumentSymbolsEngine::new(Arc::new(RwLock::new(Some(index))));
        
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = engine.handle_document_symbol(params).await?;
        
        if let Some(DocumentSymbolResponse::Nested(symbols)) = result {
            for symbol in &symbols {
                // Verify ranges are reasonable (not negative, not beyond reasonable limits)
                assert!(symbol.range.start.line < 100, "Range start line should be reasonable");
                assert!(symbol.range.end.line < 100, "Range end line should be reasonable");
                assert!(symbol.range.start.character < 1000, "Range start character should be reasonable");
                assert!(symbol.range.end.character < 1000, "Range end character should be reasonable");
                
                // Selection range should be valid
                assert!(symbol.selection_range.start.line <= symbol.selection_range.end.line,
                    "Selection range should be valid for symbol: {}", symbol.name);
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_symbols_no_index() -> Result<()> {
        // Test with uninitialized symbol index
        let engine = DocumentSymbolsEngine::new(Arc::new(RwLock::new(None)));
        
        let uri = Url::from_file_path("/test/any.gren").map_err(|_| anyhow::anyhow!("Invalid URI"))?;
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = engine.handle_document_symbol(params).await?;
        
        // Should return None when symbol index is not available
        assert!(result.is_none(), "Should return None when symbol index is not initialized");
        
        Ok(())
    }
}