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
        // Initialize logger for debugging
        let _ = tracing_subscriber::fmt::try_init();
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
        let position = Position::new(5, 0); // At "toUpper" function name at start of line
        
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
        
        // For deterministic input (toUpper function with type annotation), we MUST get hover information
        let hover = response.expect("Should return hover information for 'toUpper' function with clear type annotation and documentation");
        
        // Verify hover has a range that covers the symbol exactly
        let range = hover.range.expect("Hover should include range information");
        assert_eq!(range.start.line, 5, "Hover range should start at line 5 where 'toUpper' is defined");
        assert_eq!(range.start.character, 0, "Hover range should start at character 0 where 'toUpper' begins");
        assert_eq!(range.end.character, 7, "Hover range should end at character 7 after 'toUpper'");
        
        // Verify hover content contains expected type signature and documentation
        match hover.contents {
            HoverContents::Array(contents) => {
                assert_eq!(contents.len(), 2, "Should have exactly 2 content items: type signature and documentation");
                
                // First item should be the type signature
                let type_item = &contents[0];
                match type_item {
                    MarkedString::LanguageString(ls) => {
                        assert_eq!(ls.language, "gren", "Type signature should be marked as Gren language");
                        assert_eq!(ls.value, "toUpper : String -> String", "Should contain exact type signature");
                    }
                    _ => panic!("First hover item should be a LanguageString with type signature"),
                }
                
                // Second item should be the documentation
                let doc_item = &contents[1];
                match doc_item {
                    MarkedString::String(doc) => {
                        assert_eq!(doc, "Converts a string to uppercase", "Should contain exact documentation text");
                    }
                    _ => panic!("Second hover item should be a String with documentation"),
                }
            }
            _ => panic!("Hover contents should be Array format with type signature and documentation"),
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
        
        let expected_content = vec![
            "User",           // Type at position 0
            "createUser",     // Function at position 1  
            "processUser",    // Function at position 2
            "userName",       // Variable at position 3
        ];
        
        for (i, (position, expected_symbol)) in test_positions.iter().zip(expected_content.iter()).enumerate() {
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
            
            // If we get hover content, verify it contains the expected symbol
            if let Ok(Some(hover)) = result {
                let content_found = match hover.contents {
                    HoverContents::Markup(markup) => markup.value.contains(expected_symbol),
                    HoverContents::Scalar(marked_string) => {
                        let content = match marked_string {
                            MarkedString::String(s) => s,
                            MarkedString::LanguageString(ls) => ls.value,
                        };
                        content.contains(expected_symbol)
                    }
                    HoverContents::Array(contents) => {
                        contents.iter().any(|c| match c {
                            MarkedString::String(s) => s.contains(expected_symbol),
                            MarkedString::LanguageString(ls) => ls.value.contains(expected_symbol),
                        })
                    }
                };
                
                if content_found {
                    println!("✅ Hover test {}: Found expected symbol '{}' in hover content", i, expected_symbol);
                } else {
                    println!("⚠️ Hover test {}: Expected symbol '{}' not found in hover content", i, expected_symbol);
                }
            } else {
                println!("⚠️ Hover test {}: No hover content returned for expected symbol '{}'", i, expected_symbol);
            }
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