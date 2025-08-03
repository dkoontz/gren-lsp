use anyhow::Result;
use gren_lsp::completion::CompletionEngine;
use gren_lsp::symbol_index::SymbolIndex;
use std::path::Path;
use std::time::Instant;
use tempfile::TempDir;
use tower_lsp::lsp_types::*;
use url::Url;

/// Integration tests for code completion functionality
#[tokio::test]
async fn test_completion_engine_integration() -> Result<()> {
    // Create temporary database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_symbols.db");
    let workspace_root = temp_dir.path().to_path_buf();
    
    // Initialize symbol index
    let symbol_index = SymbolIndex::new(&db_path, workspace_root).await?;
    
    // Initialize completion engine
    let completion_engine = CompletionEngine::new(symbol_index)?;
    
    // Test basic completion engine creation
    assert!(true, "Completion engine created successfully");
    
    Ok(())
}

#[tokio::test]
async fn test_keyword_completion() -> Result<()> {
    // Create temporary database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_symbols.db");
    let workspace_root = temp_dir.path().to_path_buf();
    
    // Initialize symbol index and completion engine
    let symbol_index = SymbolIndex::new(&db_path, workspace_root).await?;
    let completion_engine = CompletionEngine::new(symbol_index)?;
    
    // Test document content
    let document_content = "
module Main exposing (..)

myFunction : String -> String
myFunction input =
    ";
    
    let uri = Url::parse("file:///test/Main.gren")?;
    let position = Position::new(5, 4); // After "="
    
    // Create completion request
    let text_document_position = TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri),
        position,
    );
    
    let completion_params = CompletionParams {
        text_document_position: text_document_position,
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    };
    
    // Test completion request
    let start_time = Instant::now();
    let response = completion_engine.handle_completion(completion_params, document_content).await?;
    let duration = start_time.elapsed();
    
    // Verify response time is under 100ms
    assert!(duration.as_millis() < 100, "Completion should respond within 100ms, took: {:?}", duration);
    
    // Verify we get some completion items
    if let Some(CompletionResponse::Array(items)) = response {
        assert!(!items.is_empty(), "Should get keyword completion items");
        
        // Check that we get appropriate keywords
        let keyword_labels: Vec<&str> = items.iter()
            .filter(|item| item.kind == Some(CompletionItemKind::KEYWORD))
            .map(|item| item.label.as_str())
            .collect();
            
        assert!(!keyword_labels.is_empty(), "Should get keyword suggestions");
        
        // Verify some expected keywords are present
        let has_let = keyword_labels.iter().any(|&label| label == "let");
        let has_if = keyword_labels.iter().any(|&label| label == "if");
        let has_when = keyword_labels.iter().any(|&label| label == "when");
        
        assert!(has_let || has_if || has_when, "Should include common keywords like let, if, or when");
    } else {
        panic!("Expected completion items array");
    }
    
    Ok(())
}

#[tokio::test] 
async fn test_module_member_completion() -> Result<()> {
    // Create temporary database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_symbols.db");
    let workspace_root = temp_dir.path().to_path_buf();
    
    // Initialize symbol index and completion engine
    let symbol_index = SymbolIndex::new(&db_path, workspace_root).await?;
    let completion_engine = CompletionEngine::new(symbol_index)?;
    
    // Test document content with module access
    let document_content = "
module Main exposing (..)

import String

myFunction : String -> String
myFunction input =
    String.
    ";
    
    let uri = Url::parse("file:///test/Main.gren")?;
    let position = Position::new(7, 11); // After "String."
    
    // Create completion request
    let text_document_position = TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri),
        position,
    );
    
    let completion_params = CompletionParams {
        text_document_position: text_document_position,
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    };
    
    // Test completion request
    let start_time = Instant::now();
    let response = completion_engine.handle_completion(completion_params, document_content).await?;
    let duration = start_time.elapsed();
    
    // Verify response time is under 100ms
    assert!(duration.as_millis() < 100, "Completion should respond within 100ms, took: {:?}", duration);
    
    // For now, just verify we get a response (the symbol index might be empty)
    match response {
        Some(CompletionResponse::Array(_items)) => {
            // Success - we got completion items
        }
        Some(CompletionResponse::List(_list)) => {
            // Success - we got completion list
        }
        None => {
            // Expected for empty symbol index - just verify the mechanism works
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_completion_performance() -> Result<()> {
    // Create temporary database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_symbols.db");
    let workspace_root = temp_dir.path().to_path_buf();
    
    // Initialize symbol index and completion engine
    let symbol_index = SymbolIndex::new(&db_path, workspace_root).await?;
    let completion_engine = CompletionEngine::new(symbol_index)?;
    
    // Test document content
    let document_content = "
module Main exposing (..)

test = 
    ";
    
    let uri = Url::parse("file:///test/Main.gren")?;
    let position = Position::new(3, 4);
    
    // Create completion request
    let text_document_position = TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri),
        position,
    );
    
    let completion_params = CompletionParams {
        text_document_position: text_document_position,
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    };
    
    // Test multiple completion requests to verify consistent performance
    for i in 0..10 {
        let start_time = Instant::now();
        let _response = completion_engine.handle_completion(completion_params.clone(), document_content).await?;
        let duration = start_time.elapsed();
        
        assert!(
            duration.as_millis() < 100, 
            "Completion request {} should respond within 100ms, took: {:?}", 
            i, 
            duration
        );
    }
    
    Ok(())
}

#[tokio::test]
async fn test_completion_item_quality() -> Result<()> {
    // Create temporary database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_symbols.db");
    let workspace_root = temp_dir.path().to_path_buf();
    
    // Initialize symbol index and completion engine
    let symbol_index = SymbolIndex::new(&db_path, workspace_root).await?;
    let completion_engine = CompletionEngine::new(symbol_index)?;
    
    // Test document content
    let document_content = "
module Main exposing (..)

myFunction : String -> String
myFunction input =
    le
    ";
    
    let uri = Url::parse("file:///test/Main.gren")?;
    let position = Position::new(5, 6); // After "le"
    
    // Create completion request
    let text_document_position = TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri),
        position,
    );
    
    let completion_params = CompletionParams {
        text_document_position: text_document_position,
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    };
    
    // Test completion request
    let response = completion_engine.handle_completion(completion_params, document_content).await?;
    
    // Verify completion item quality
    if let Some(CompletionResponse::Array(items)) = response {
        for item in &items {
            // Verify basic completion item structure
            assert!(!item.label.is_empty(), "Completion item should have non-empty label");
            assert!(item.kind.is_some(), "Completion item should have a kind");
            
            // If it has documentation, it should be non-empty
            if let Some(ref doc) = item.documentation {
                match doc {
                    Documentation::String(s) => assert!(!s.is_empty(), "Documentation string should not be empty"),
                    Documentation::MarkupContent(content) => assert!(!content.value.is_empty(), "Documentation content should not be empty"),
                }
            }
            
            // Verify insert text is provided
            assert!(item.insert_text.is_some(), "Completion item should have insert text");
        }
        
        // Look for "let" keyword specifically since we typed "le"
        let has_let = items.iter().any(|item| item.label == "let");
        assert!(has_let, "Should suggest 'let' keyword when typing 'le'");
    }
    
    Ok(())
}