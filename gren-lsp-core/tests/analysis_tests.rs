use gren_lsp_core::Workspace;
use lsp_types::*;
use std::time::Instant;

/// Create a test document with Gren content
fn create_test_document(uri: &str, content: &str, version: i32) -> TextDocumentItem {
    TextDocumentItem {
        uri: Url::parse(uri).unwrap(),
        language_id: "gren".to_string(),
        version,
        text: content.to_string(),
    }
}

/// Test: Analysis Workflow Coordination
/// Purpose: Test complete document analysis pipeline: parse → symbols → diagnostics
#[tokio::test]
async fn test_complete_analysis_workflow() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // Input: Document with function definition
    let content = r#"module Test exposing (greet)

greet : String -> String  
greet name =
    "Hello, " ++ name"#;

    let text_document = create_test_document("file:///test.gren", content, 1);
    let uri = text_document.uri.clone();

    // Execute: Complete analysis workflow
    workspace.open_document(text_document).unwrap();

    // Expected: Document exists in workspace
    assert!(workspace.is_document_open(&uri));
    
    // Get the document for validation
    let document = workspace.get_document(&uri).unwrap();
    assert_eq!(document.text(), content);
    assert_eq!(document.version(), 1);

    // Expected: Symbols were extracted and indexed
    let symbols = workspace.get_file_symbols(&uri).unwrap_or_default();
    
    // Should find the greet function
    let greet_symbol = symbols.iter().find(|s| s.name == "greet");
    assert!(greet_symbol.is_some(), "Should find 'greet' function symbol");

    // Expected: Basic diagnostics (syntax checking)
    let diagnostics = workspace.get_diagnostics(&uri);
    // For valid Gren code, should have no syntax errors
    let error_diagnostics: Vec<_> = diagnostics.iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(error_diagnostics.is_empty(), "Valid Gren code should have no syntax errors");
}

/// Test: Analysis Result Caching
/// Purpose: Verify cached results returned when content unchanged
#[tokio::test]
async fn test_analysis_caching() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    let content = r#"module CacheTest exposing (..)

helper : String -> String
helper x = x ++ "!"

main = helper "test""#;

    let text_document = create_test_document("file:///cache_test.gren", content, 1);
    let uri = text_document.uri.clone();

    // First analysis
    let start_time1 = Instant::now();
    workspace.open_document(text_document).unwrap();
    let symbols1 = workspace.get_file_symbols(&uri).unwrap_or_default();
    let analysis_time1 = start_time1.elapsed();

    // Second analysis (same content - should use cache)  
    let start_time2 = Instant::now();
    let symbols2 = workspace.get_file_symbols(&uri).unwrap_or_default();
    let analysis_time2 = start_time2.elapsed();

    // Expected: Same results
    assert_eq!(symbols1.len(), symbols2.len());
    
    // Find corresponding symbols
    for symbol1 in &symbols1 {
        let symbol2 = symbols2.iter().find(|s| s.name == symbol1.name);
        assert!(symbol2.is_some(), "Symbol '{}' should exist in cached results", symbol1.name);
        let symbol2 = symbol2.unwrap();
        assert_eq!(symbol1.kind, symbol2.kind);
        assert_eq!(symbol1.location, symbol2.location);
    }

    // Second analysis should be faster (cached)
    // Note: This is a basic check - in practice cache hits can be very fast
    assert!(analysis_time2 <= analysis_time1, "Cached analysis should not be slower");
}

/// Test: Cross-File Analysis
/// Purpose: Test import resolution and dependency tracking
#[tokio::test] 
async fn test_cross_file_import_resolution() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // File 1: Utils.gren
    let utils_content = r#"module Utils exposing (helper)

helper : String -> String
helper x = x ++ "!""#;

    let utils_document = create_test_document("file:///Utils.gren", utils_content, 1);
    let utils_uri = utils_document.uri.clone();

    // File 2: Main.gren  
    let main_content = r#"module Main exposing (main)

import Utils

main = Utils.helper "test""#;

    let main_document = create_test_document("file:///Main.gren", main_content, 1);
    let main_uri = main_document.uri.clone();

    // Add both documents to workspace
    workspace.open_document(utils_document).unwrap();
    workspace.open_document(main_document).unwrap();

    // Expected: Both documents analyzed successfully
    assert!(workspace.is_document_open(&utils_uri));
    assert!(workspace.is_document_open(&main_uri));

    // Expected: Symbols extracted from both files
    let utils_symbols = workspace.get_file_symbols(&utils_uri).unwrap_or_default();
    let main_symbols = workspace.get_file_symbols(&main_uri).unwrap_or_default();

    // Utils.gren should have helper function
    let helper_symbol = utils_symbols.iter().find(|s| s.name == "helper");
    assert!(helper_symbol.is_some(), "Should find 'helper' function in Utils.gren");

    // Main.gren should have main function
    let main_symbol = main_symbols.iter().find(|s| s.name == "main");
    assert!(main_symbol.is_some(), "Should find 'main' function in Main.gren");

    // Expected: No import errors (basic syntax validation)
    let main_diagnostics = workspace.get_diagnostics(&main_uri);
    let import_errors: Vec<_> = main_diagnostics.iter()
        .filter(|d| d.message.contains("import") && d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    // Note: Full import resolution requires compiler integration, so we only check for basic syntax errors
    assert!(import_errors.is_empty() || main_diagnostics.is_empty(), "Should not have syntax errors in import statements");
}

/// Test: Document Analysis with Syntax Errors
/// Purpose: Test analysis behavior with invalid Gren syntax
#[tokio::test]
async fn test_analysis_with_syntax_errors() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // Input: Document with syntax errors
    let invalid_content = r#"module Invalid exposing (..)

-- Missing function body
invalidFunction : String ->

-- Unbalanced parentheses
calculate x =
    (x + 1 * 2

-- Missing type annotation (warning, not error)
noTypeAnnotation = "test""#;

    let text_document = create_test_document("file:///invalid.gren", invalid_content, 1);
    let uri = text_document.uri.clone();

    // Execute: Analysis should handle errors gracefully
    workspace.open_document(text_document).unwrap();

    // Expected: Document exists despite syntax errors
    assert!(workspace.is_document_open(&uri));
    
    let document = workspace.get_document(&uri).unwrap();
    assert_eq!(document.text(), invalid_content);

    // Expected: Diagnostics should report syntax errors
    let diagnostics = workspace.get_diagnostics(&uri);
    assert!(!diagnostics.is_empty(), "Should report diagnostics for syntax errors");
    
    // Should have at least one error diagnostic
    let error_count = diagnostics.iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .count();
    assert!(error_count > 0, "Should report at least one syntax error");
}

/// Test: Incremental Analysis Updates
/// Purpose: Test analysis updates when document content changes
#[tokio::test]
async fn test_incremental_analysis_updates() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // Initial content
    let initial_content = r#"module Incremental exposing (..)

oldFunction : String -> String
oldFunction x = x ++ "old""#;

    let text_document = create_test_document("file:///incremental.gren", initial_content, 1);
    let uri = text_document.uri.clone();

    // Initial analysis
    workspace.open_document(text_document).unwrap();
    
    let initial_symbols = workspace.get_file_symbols(&uri).unwrap_or_default();
    let old_function_exists = initial_symbols.iter().any(|s| s.name == "oldFunction");
    assert!(old_function_exists, "Should find 'oldFunction' initially");

    // Update document content
    let updated_content = r#"module Incremental exposing (..)

newFunction : String -> String  
newFunction x = x ++ "new"

anotherFunction : Int -> Int
anotherFunction n = n * 2"#;

    let change = TextDocumentContentChangeEvent {
        range: None, // Full document replacement
        range_length: None,
        text: updated_content.to_string(),
    };

    // Apply change
    workspace.update_document(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![change],
    }).unwrap();

    // Expected: Document content updated
    let document = workspace.get_document(&uri).unwrap();
    assert_eq!(document.text(), updated_content);
    assert_eq!(document.version(), 2);

    // Expected: Symbols updated to reflect changes
    let updated_symbols = workspace.get_file_symbols(&uri).unwrap_or_default();
    
    let new_function_exists = updated_symbols.iter().any(|s| s.name == "newFunction");
    let another_function_exists = updated_symbols.iter().any(|s| s.name == "anotherFunction");
    let old_function_still_exists = updated_symbols.iter().any(|s| s.name == "oldFunction");
    
    assert!(new_function_exists, "Should find 'newFunction' after update");
    assert!(another_function_exists, "Should find 'anotherFunction' after update");
    assert!(!old_function_still_exists, "Should not find 'oldFunction' after replacement");
}

/// Test: Analysis Performance with Large Documents
/// Purpose: Test analysis handles large documents efficiently
#[tokio::test]
async fn test_analysis_large_document() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // Generate large document content
    let mut large_content = String::from("module Large exposing (..)\n\n");
    for i in 0..200 {
        large_content.push_str(&format!(
            "function{} : Int -> Int\nfunction{} x = x + {}\n\n",
            i, i, i
        ));
    }
    
    let text_document = create_test_document("file:///large.gren", &large_content, 1);
    let uri = text_document.uri.clone();

    // Measure analysis performance
    let start_time = Instant::now();
    workspace.open_document(text_document).unwrap();
    let analysis_duration = start_time.elapsed();

    // Expected: Analysis completes in reasonable time (under 2 seconds for 200 functions)
    assert!(analysis_duration.as_secs() < 2, "Large document analysis should complete quickly");

    // Expected: All symbols extracted
    let symbols = workspace.get_file_symbols(&uri).unwrap_or_default();
    assert!(symbols.len() >= 200, "Should extract all function symbols from large document");
    
    // Verify specific functions exist
    let function0_exists = symbols.iter().any(|s| s.name == "function0");
    let function199_exists = symbols.iter().any(|s| s.name == "function199");
    assert!(function0_exists, "Should find first function symbol");
    assert!(function199_exists, "Should find last function symbol");

    // Expected: Document processed correctly
    let document = workspace.get_document(&uri).unwrap();
    assert_eq!(document.text().len(), large_content.len());
}

/// Test: Symbol Extraction Accuracy
/// Purpose: Test symbol extraction finds all expected symbol types
#[tokio::test]
async fn test_symbol_extraction_accuracy() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // Content with various symbol types
    let content = r#"module SymbolTest exposing (..)

import Array

type alias User =
    { name : String
    , age : Int
    }

type Message
    = Success String
    | Error String
    | Info

constant : Int
constant = 42

greet : User -> String
greet user = "Hello, " ++ user.name

processMessage : Message -> String
processMessage msg =
    when msg is
        Success data -> data
        Error err -> "Error: " ++ err
        Info -> "Information message""#;

    let text_document = create_test_document("file:///symbol_test.gren", content, 1);
    let uri = text_document.uri.clone();

    workspace.open_document(text_document).unwrap();
    
    let symbols = workspace.get_file_symbols(&uri).unwrap_or_default();
    assert!(!symbols.is_empty(), "Should extract symbols from document");

    // Expected symbol names - check that symbols are found
    let expected_symbols = vec!["User", "Message", "constant", "greet", "processMessage"];

    for name in expected_symbols {
        let symbol = symbols.iter().find(|s| s.name == name);
        assert!(symbol.is_some(), "Should find symbol '{}'", name);
    }
}

/// Test: Analysis Error Recovery
/// Purpose: Test analysis continues after encountering errors
#[tokio::test]
async fn test_analysis_error_recovery() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // Mix of valid and invalid content
    let mixed_content = r#"module ErrorRecovery exposing (..)

-- Valid function
validFunction : String -> String
validFunction x = x ++ "!"

-- Invalid syntax 1: Missing body
invalidFunction1 : String ->

-- Valid function after error
anotherValidFunction : Int -> Int
anotherValidFunction n = n * 2

-- Invalid syntax 2: Unbalanced braces
invalidFunction2 = {
    "missing closing brace"

-- Valid function at end
finalFunction : String -> String
finalFunction s = "Final: " ++ s"#;

    let text_document = create_test_document("file:///mixed.gren", mixed_content, 1);
    let uri = text_document.uri.clone();

    // Analysis should complete despite errors
    workspace.open_document(text_document).unwrap();
    
    // Expected: Document processed
    assert!(workspace.is_document_open(&uri));

    // Expected: Valid symbols still extracted
    let symbols = workspace.get_file_symbols(&uri).unwrap_or_default();
    
    let valid_symbols = ["validFunction", "anotherValidFunction", "finalFunction"];
    for symbol_name in valid_symbols {
        let found = symbols.iter().any(|s| s.name == symbol_name);
        assert!(found, "Should extract valid symbol '{}' despite nearby syntax errors", symbol_name);
    }

    // Expected: Errors reported in diagnostics
    let diagnostics = workspace.get_diagnostics(&uri);
    let error_count = diagnostics.iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .count();
    assert!(error_count > 0, "Should report syntax errors in diagnostics");
}

/// Test: Workspace Statistics
/// Purpose: Test workspace provides accurate statistics
#[tokio::test]
async fn test_workspace_statistics() {
    let mut workspace = Workspace::new().expect("Failed to create workspace");
    
    // Initially empty
    let initial_stats = workspace.stats();
    assert_eq!(initial_stats.document_count, 0);
    assert!(initial_stats.open_documents.is_empty());

    // Add documents
    let doc1 = create_test_document("file:///doc1.gren", "module Doc1 exposing (..)", 1);
    let doc2 = create_test_document("file:///doc2.gren", "module Doc2 exposing (..)", 1);
    let doc3 = create_test_document("file:///doc3.gren", "module Doc3 exposing (..)", 1);
    
    let uri1 = doc1.uri.clone();
    let uri2 = doc2.uri.clone();
    let uri3 = doc3.uri.clone();

    workspace.open_document(doc1).unwrap();
    workspace.open_document(doc2).unwrap();
    workspace.open_document(doc3).unwrap();

    // Check statistics after adding documents
    let stats = workspace.stats();
    assert_eq!(stats.document_count, 3);
    assert_eq!(stats.open_documents.len(), 3);
    assert!(stats.open_documents.contains(&uri1));
    assert!(stats.open_documents.contains(&uri2));
    assert!(stats.open_documents.contains(&uri3));

    // Close one document
    workspace.close_document(uri2.clone()).unwrap();

    // Check statistics after closing
    let final_stats = workspace.stats();
    assert_eq!(final_stats.document_count, 2);
    assert_eq!(final_stats.open_documents.len(), 2);
    assert!(final_stats.open_documents.contains(&uri1));
    assert!(!final_stats.open_documents.contains(&uri2));
    assert!(final_stats.open_documents.contains(&uri3));
}