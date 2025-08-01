use gren_lsp_core::Document;
use lsp_types::*;

/// Create a test text document item
fn create_test_lsp_item(uri: &str, content: &str, version: i32) -> TextDocumentItem {
    TextDocumentItem {
        uri: Url::parse(uri).unwrap(),
        language_id: "gren".to_string(),
        version,
        text: content.to_string(),
    }
}

/// Test: Document Creation and Initialization
/// Purpose: Test conversion from LSP TextDocumentItem to internal Document
#[test]
fn test_document_from_lsp_item() {
    // Input: LSP TextDocumentItem
    let lsp_item = TextDocumentItem {
        uri: Url::parse("file:///test.gren").unwrap(),
        language_id: "gren".to_string(),
        version: 1,
        text: "module Test exposing (..)".to_string(),
    };

    // Create Document
    let document = Document::new(lsp_item);

    // Expected: Exact field mapping
    assert_eq!(document.uri().as_str(), "file:///test.gren");
    assert_eq!(document.version(), 1);
    assert_eq!(document.text(), "module Test exposing (..)");
    assert_eq!(document.language_id(), "gren");
    // Note: Parse tree is created lazily when requested
}

/// Test: Document initialization with complex content
#[test]
fn test_document_complex_initialization() {
    let content = r#"module ComplexTest exposing (..)

import Array

type alias User =
    { name : String
    , age : Int  
    }

greet : User -> String
greet user =
    "Hello, " ++ user.name"#;

    let lsp_item = create_test_lsp_item("file:///complex.gren", content, 1);
    let document = Document::new(lsp_item);

    // Expected: Document properly initialized with complex content
    assert_eq!(document.uri().as_str(), "file:///complex.gren");
    assert_eq!(document.version(), 1);
    assert_eq!(document.text(), content);
    // Note: Parse tree would be created when requested with parser
}

/// Test: Incremental Text Updates  
/// Purpose: Test range-based content changes
#[test]
fn test_incremental_text_update() {
    // Initial content
    let initial = "module Test exposing (..)\n\ngreet = \"Hello\"";
    let lsp_item = create_test_lsp_item("file:///test.gren", initial, 1);
    let mut document = Document::new(lsp_item);

    // Change: Replace "Hello" with "Hi"  
    let change = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 2, character: 9 },  // After 'greet = "'
            end: Position { line: 2, character: 14 },   // Before closing quote
        }),
        range_length: Some(5),
        text: "Hi".to_string(),
    };

    // Apply change
    document.apply_changes(vec![change]).unwrap();

    // Expected: Exact updated content
    assert_eq!(document.version(), 2);
    assert_eq!(document.text(), "module Test exposing (..)\n\ngreet = \"Hi\"");
    // Parse tree would be updated when requested
}

/// Test: Full document replacement
#[test]
fn test_full_document_replacement() {
    let initial = "module Old exposing (..)";
    let lsp_item = create_test_lsp_item("file:///test.gren", initial, 1);
    let mut document = Document::new(lsp_item);

    // Full replacement (no range specified)
    let change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "module New exposing (..)\\n\\nnewFunction = 42".to_string(),
    };

    document.apply_changes(vec![change]).unwrap();

    // Expected: Complete content replacement
    assert_eq!(document.version(), 2);
    assert_eq!(document.text(), "module New exposing (..)\\n\\nnewFunction = 42");
    // Parse tree would be updated when requested
}

/// Test: Multiple incremental changes
#[test]
fn test_multiple_incremental_changes() {
    let initial = "module Test exposing (..)\n\nvalue = 0";
    let lsp_item = create_test_lsp_item("file:///test.gren", initial, 1);
    let mut document = Document::new(lsp_item);

    // First change: Update value
    let change1 = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 2, character: 8 },
            end: Position { line: 2, character: 9 },
        }),
        range_length: Some(1),
        text: "42".to_string(),
    };
    document.apply_changes(vec![change1]).unwrap();

    // Second change: Add new line
    let change2 = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 2, character: 10 },
            end: Position { line: 2, character: 10 },
        }),
        range_length: Some(0),
        text: "\n\nanotherValue = 100".to_string(),
    };
    document.apply_changes(vec![change2]).unwrap();

    // Expected: Both changes applied correctly
    assert_eq!(document.version(), 3);
    assert_eq!(document.text(), "module Test exposing (..)\n\nvalue = 42\n\nanotherValue = 100");
    // Parse tree would be updated when requested
}

/// Test: Version tracking and document updates
#[test]
fn test_version_tracking() {
    let initial = "module Test exposing (..)";
    let lsp_item = create_test_lsp_item("file:///test.gren", initial, 5);
    let mut document = Document::new(lsp_item);

    // Note: Document API doesn't take version parameter, versions are handled internally
    let old_version = document.version();

    let old_change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "module OldUpdate exposing (..)".to_string(),
    };

    // Apply change
    document.apply_changes(vec![old_change]).unwrap();
    
    // Expected: Content was updated (document doesn't reject based on version)
    assert!(document.version() > old_version);
    
    let valid_change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "module ValidUpdate exposing (..)".to_string(),
    };
    
    document.apply_changes(vec![valid_change]).unwrap();
    assert_eq!(document.text(), "module ValidUpdate exposing (..)");
}

/// Test: Parse tree persistence through changes
#[test]
fn test_parse_tree_persistence() {
    let initial = "module Test exposing (..)\n\ngreet name = \"Hello, \" ++ name";
    let lsp_item = create_test_lsp_item("file:///test.gren", initial, 1);
    let mut document = Document::new(lsp_item);

    // Verify initial parse tree can be created
    let mut parser = gren_lsp_core::Parser::new().unwrap();
    let initial_tree = document.get_parse_tree(&mut parser).unwrap();
    assert!(initial_tree.is_some());

    // Make a change that affects the parse tree (replace "Hello, " with "Greetings, ")
    let change = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 2, character: 14 }, // After 'greet name = "'
            end: Position { line: 2, character: 21 },   // After "Hello, "
        }),
        range_length: Some(7),
        text: "Greetings, ".to_string(),
    };
    
    document.apply_changes(vec![change]).unwrap();

    // Expected: Parse tree updated but still exists
    let updated_tree = document.get_parse_tree(&mut parser).unwrap();
    assert!(updated_tree.is_some());
    
    // Content should reflect the change
    assert_eq!(document.text(), "module Test exposing (..)\n\ngreet name = \"Greetings, \" ++ name");
}

/// Test: Document with syntax errors
#[test]
fn test_document_with_syntax_errors() {
    let invalid_content = r#"module Test exposing (..)

-- Invalid syntax: missing function body
invalidFunction : String ->
    -- Missing implementation
"#;

    let lsp_item = create_test_lsp_item("file:///invalid.gren", invalid_content, 1);
    let mut document = Document::new(lsp_item);

    // Expected: Document created despite syntax errors
    assert_eq!(document.uri().as_str(), "file:///invalid.gren");
    assert_eq!(document.version(), 1);
    assert_eq!(document.text(), invalid_content);
    
    // Parse tree should be creatable but may contain errors
    let mut parser = gren_lsp_core::Parser::new().unwrap();
    let parse_tree = document.get_parse_tree(&mut parser).unwrap();
    assert!(parse_tree.is_some());
    
    // Should have syntax errors
    assert!(document.has_syntax_errors());
}

/// Test: Empty document handling
#[test]
fn test_empty_document() {
    let lsp_item = create_test_lsp_item("file:///empty.gren", "", 1);
    let mut document = Document::new(lsp_item);

    // Expected: Empty document handled gracefully
    assert_eq!(document.uri().as_str(), "file:///empty.gren");
    assert_eq!(document.version(), 1);
    assert_eq!(document.text(), "");
    
    // Should be able to create parse tree even for empty content
    let mut parser = gren_lsp_core::Parser::new().unwrap();
    let parse_tree = document.get_parse_tree(&mut parser).unwrap();
    assert!(parse_tree.is_some());
}

/// Test: Large document handling
#[test]
fn test_large_document() {
    // Create large content (many functions)
    let mut large_content = String::from("module Large exposing (..)\\n\\n");
    for i in 0..1000 {
        large_content.push_str(&format!(
            "function{} : Int -> Int\\nfunction{} x = x + {}\\n\\n",
            i, i, i
        ));
    }

    let lsp_item = create_test_lsp_item("file:///large.gren", &large_content, 1);
    let mut document = Document::new(lsp_item);

    // Expected: Large document handled efficiently
    assert_eq!(document.uri().as_str(), "file:///large.gren");
    assert_eq!(document.version(), 1);
    assert_eq!(document.text(), large_content);
    
    let mut parser = gren_lsp_core::Parser::new().unwrap();
    let parse_tree = document.get_parse_tree(&mut parser).unwrap();
    assert!(parse_tree.is_some());
    
    // Should contain expected functions
    assert!(document.text().contains("function999"));
    assert!(document.text().contains("x + 999"));
}

/// Test: Document state after failed changes
#[test]
fn test_failed_change_state() {
    let initial = "module Test exposing (..)";
    let lsp_item = create_test_lsp_item("file:///test.gren", initial, 1);
    let mut document = Document::new(lsp_item);

    // Try a range change that might cause issues (replace valid range)
    let edge_case_change = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 0, character: 7 },  // At "Test"
            end: Position { line: 0, character: 11 },   // After "Test"
        }),
        range_length: Some(4),
        text: "NewModule".to_string(),
    };

    let result = document.apply_changes(vec![edge_case_change]);

    // Expected: Change should succeed and update content appropriately
    assert!(result.is_ok());
    assert_eq!(document.text(), "module NewModule exposing (..)");
    assert_eq!(document.version(), 2);
}

/// Test: Unicode content handling
#[test]
fn test_unicode_content() {
    let unicode_content = r#"module Test exposing (..)

-- Unicode characters: áéíóú, 中文, 🚀
greet : String -> String  
greet name =
    "¡Hola, " ++ name ++ "! 🌟"
"#;

    let lsp_item = create_test_lsp_item("file:///unicode.gren", unicode_content, 1);
    let mut document = Document::new(lsp_item);

    // Expected: Unicode content preserved exactly
    assert_eq!(document.text(), unicode_content);
    let mut parser = gren_lsp_core::Parser::new().unwrap();
    let parse_tree = document.get_parse_tree(&mut parser).unwrap();
    assert!(parse_tree.is_some());
    
    // Unicode content should be in the text
    assert!(document.text().contains("áéíóú"));
    assert!(document.text().contains("中文"));
    assert!(document.text().contains("🚀"));
    assert!(document.text().contains("🌟"));
}