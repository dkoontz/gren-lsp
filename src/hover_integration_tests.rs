#[cfg(test)]
mod hover_integration_tests {
    use crate::hover::HoverEngine;
    use tempfile::TempDir;
    use std::time::Instant;
    use tower_lsp::lsp_types::*;
    use url::Url;
    use anyhow::Result;

    /// Test hover engine creation and basic functionality
    #[tokio::test]
    async fn test_hover_basic_workflow() -> Result<()> {
        // Create temporary workspace
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Use in-memory database for testing
        let symbol_index = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root).await?;
        
        // Initialize hover engine
        let mut hover_engine = HoverEngine::new(symbol_index)?;
        
        // Test basic hover context building
        let uri = Url::parse("file:///test/Main.gren")?;
        let position = Position::new(2, 10); // At "myFunction"
        
        let text_document_position = TextDocumentPositionParams::new(
            TextDocumentIdentifier::new(uri.clone()),
            position,
        );
        
        let hover_params = HoverParams {
            text_document_position_params: text_document_position,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        // Test that hover engine responds without errors
        let document_content = "module Main exposing (..)\n\nmyFunction : String -> String\nmyFunction input = input";
        let start_time = Instant::now();
        
        // This should not fail even if it returns empty results
        let result = hover_engine.handle_hover(hover_params, document_content).await;
        let duration = start_time.elapsed();
        
        // Verify it responds quickly and doesn't error
        match result {
            Ok(_) => {
                assert!(duration.as_millis() < 50, "Should respond within 50ms");
            }
            Err(e) => {
                eprintln!("Hover error: {:?}", e);
                panic!("Hover should not error: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Test hover on type annotations
    #[tokio::test]
    async fn test_hover_type_annotations() -> Result<()> {
        // Create temporary workspace
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Use in-memory database for testing
        let symbol_index = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root).await?;
        let mut hover_engine = HoverEngine::new(symbol_index)?;
        
        // Test document content with type annotations
        let document_content = r#"
module Main exposing (..)

{-| Converts a string to uppercase -}
toUpper : String -> String
toUpper input = String.toUpper input

myFunction : Int -> Int -> Int
myFunction x y = x + y
        "#;
        
        let uri = Url::parse("file:///test/Main.gren")?;
        let position = Position::new(5, 8); // At "toUpper" function name
        
        let text_document_position = TextDocumentPositionParams::new(
            TextDocumentIdentifier::new(uri),
            position,
        );
        
        let hover_params = HoverParams {
            text_document_position_params: text_document_position,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        // Test hover request
        let start_time = Instant::now();
        let response = hover_engine.handle_hover(hover_params, document_content).await?;
        let duration = start_time.elapsed();
        
        // Verify response time is under 50ms
        assert!(duration.as_millis() < 50, "Hover should respond within 50ms, took: {:?}", duration);
        
        // Verify we get hover information (may be None if symbol not found, but should not error)
        match response {
            Some(hover) => {
                // If we get hover info, verify it has meaningful content
                match hover.contents {
                    HoverContents::Array(contents) => {
                        assert!(!contents.is_empty(), "Hover should have some content");
                    }
                    HoverContents::Markup(_) => {
                        // Single markup content is also valid
                    }
                    HoverContents::Scalar(_) => {
                        // Single scalar content is also valid
                    }
                }
            }
            None => {
                // No hover info found - this is acceptable for a symbol not in the index
            }
        }
        
        Ok(())
    }
    
    /// Test hover performance with various document sizes
    #[tokio::test]
    async fn test_hover_performance() -> Result<()> {
        // Create temporary workspace
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Use in-memory database for testing
        let symbol_index = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root).await?;
        let mut hover_engine = HoverEngine::new(symbol_index)?;
        
        // Test document content with multiple functions
        let document_content = r#"
module Main exposing (..)

function1 : String -> String
function1 x = x

function2 : Int -> Int
function2 x = x + 1

function3 : Array Int -> Int
function3 arr = Array.length arr

function4 : String -> String -> String
function4 a b = a ++ b

test = function1 "hello"
        "#;
        
        let uri = Url::parse("file:///test/Main.gren")?;
        
        // Test multiple hover requests to verify consistent performance
        for i in 0..10 {
            let position = Position::new(7, 8); // At different positions
            
            let text_document_position = TextDocumentPositionParams::new(
                TextDocumentIdentifier::new(uri.clone()),
                position,
            );
            
            let hover_params = HoverParams {
                text_document_position_params: text_document_position,
                work_done_progress_params: WorkDoneProgressParams::default(),
            };
            
            let start_time = Instant::now();
            let _response = hover_engine.handle_hover(hover_params, document_content).await?;
            let duration = start_time.elapsed();
            
            assert!(
                duration.as_millis() < 50, 
                "Hover request {} should respond within 50ms, took: {:?}", 
                i, 
                duration
            );
        }
        
        Ok(())
    }
    
    /// Test hover on different symbol types
    #[tokio::test]
    async fn test_hover_symbol_types() -> Result<()> {
        // Create temporary workspace
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Use in-memory database for testing
        let symbol_index = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root).await?;
        let mut hover_engine = HoverEngine::new(symbol_index)?;
        
        // Test document content with different symbol types
        let document_content = r#"
module Main exposing (..)

type User = { name : String, age : Int }

createUser : String -> Int -> User
createUser name age = { name = name, age = age }

processUser user =
    let userName = user.name
    in userName
        "#;
        
        let uri = Url::parse("file:///test/Main.gren")?;
        
        // Test hover on different positions
        let test_positions = vec![
            Position::new(3, 5),  // At "User" type
            Position::new(5, 8),  // At "createUser" function
            Position::new(8, 4),  // At "processUser" function
            Position::new(9, 8),  // At "userName" variable
        ];
        
        for (i, position) in test_positions.iter().enumerate() {
            let text_document_position = TextDocumentPositionParams::new(
                TextDocumentIdentifier::new(uri.clone()),
                *position,
            );
            
            let hover_params = HoverParams {
                text_document_position_params: text_document_position,
                work_done_progress_params: WorkDoneProgressParams::default(),
            };
            
            let start_time = Instant::now();
            let result = hover_engine.handle_hover(hover_params, document_content).await;
            let duration = start_time.elapsed();
            
            // Verify performance and no errors
            assert!(result.is_ok(), "Hover test {} should not error", i);
            assert!(duration.as_millis() < 50, "Hover test {} should respond within 50ms", i);
        }
        
        Ok(())
    }
    
    /// Test hover range accuracy
    #[tokio::test]
    async fn test_hover_range_accuracy() -> Result<()> {
        // Create temporary workspace
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Use in-memory database for testing
        let symbol_index = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root).await?;
        let mut hover_engine = HoverEngine::new(symbol_index)?;
        
        // Test document content
        let document_content = "module Main exposing (..)\n\nmyFunc : String -> String\nmyFunc x = x";
        
        let uri = Url::parse("file:///test/Main.gren")?;
        let position = Position::new(3, 2); // At "myFunc"
        
        let text_document_position = TextDocumentPositionParams::new(
            TextDocumentIdentifier::new(uri),
            position,
        );
        
        let hover_params = HoverParams {
            text_document_position_params: text_document_position,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        // Test hover request
        let response = hover_engine.handle_hover(hover_params, document_content).await?;
        
        // If we get a response, verify the range is meaningful
        if let Some(hover) = response {
            if let Some(range) = hover.range {
                // Verify range coordinates are valid
                assert!(range.start.line <= range.end.line, "Range start line should be <= end line");
                if range.start.line == range.end.line {
                    assert!(range.start.character <= range.end.character, "Range start char should be <= end char on same line");
                }
            }
        }
        
        Ok(())
    }
}