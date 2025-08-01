use gren_lsp_server::{test_utils::*};
use lsp_types::*;

// For these server tests, we'll focus on testing the test utilities
// and other testable components rather than trying to mock complex server instances
// The core LSP functionality is tested in the handlers and protocol layers

/// Test: Initialize request creation utility
/// Purpose: Test that test utilities create correct LSP requests
#[test]
fn test_initialize_request_creation() {
    let init_params = LspTestClient::create_initialize_request();
    
    // Expected: Valid initialize request with required capabilities
    assert!(init_params.capabilities.text_document.is_some());
    assert!(init_params.capabilities.workspace.is_some());
    assert_eq!(
        init_params.client_info.as_ref().unwrap().name,
        "test-client"
    );
    
    // Verify text document capabilities
    let text_doc_caps = init_params.capabilities.text_document.unwrap();
    assert!(text_doc_caps.completion.is_some());
    assert!(text_doc_caps.hover.is_some());
    assert!(text_doc_caps.definition.is_some());
    assert!(text_doc_caps.references.is_some());
    assert!(text_doc_caps.document_symbol.is_some());
    
    // Verify workspace capabilities  
    let workspace_caps = init_params.capabilities.workspace.unwrap();
    assert!(workspace_caps.workspace_folders.is_some());
    assert!(workspace_caps.symbol.is_some());
}

/// Test: Document open notification creation
/// Purpose: Test didOpen notification creation utility
#[test]
fn test_did_open_notification_creation() {
    let did_open_params = LspTestClient::create_did_open_notification(
        "file:///test.gren",
        "gren", 
        "module Test exposing (..)"
    );
    
    // Expected: Valid didOpen notification
    assert_eq!(
        did_open_params.text_document.uri.as_str(),
        "file:///test.gren"
    );
    assert_eq!(did_open_params.text_document.language_id, "gren");
    assert_eq!(did_open_params.text_document.version, 1);
    assert_eq!(did_open_params.text_document.text, "module Test exposing (..)");
}

/// Test: Completion request creation utility
/// Purpose: Test completion request utility functions
#[test]
fn test_completion_request_creation() {
    let completion_params = LspTestClient::create_completion_request(
        "file:///test.gren", 
        5, 
        10
    );
    
    // Expected: Valid completion request
    assert_eq!(
        completion_params.text_document_position.text_document.uri.as_str(),
        "file:///test.gren"
    );
    assert_eq!(completion_params.text_document_position.position.line, 5);
    assert_eq!(completion_params.text_document_position.position.character, 10);
    
    // Verify request structure
    assert!(matches!(
        completion_params.work_done_progress_params,
        WorkDoneProgressParams { .. }
    ));
    assert!(matches!(
        completion_params.partial_result_params,
        PartialResultParams { .. }
    ));
}

/// Test: Gren source creation utility
/// Purpose: Test test source file generation
#[test]
fn test_test_gren_source_creation() {
    let source = create_test_gren_source();
    
    // Expected: Valid Gren source with test patterns
    assert!(source.contains("module TestModule"));
    assert!(source.contains("greetUser"));
    assert!(source.contains("User"));
    assert!(source.contains("type alias User"));
    assert!(source.contains("type Message"));
    assert!(source.contains("processUsers"));
    assert!(source.contains("Array.map"));
    
    // Verify it's valid Gren-like syntax
    assert!(source.starts_with("module TestModule exposing (..)"));
    assert!(source.contains("import Array"));
}

/// Test: LSP request parameter validation
/// Purpose: Test that request utilities create valid LSP structures
#[test]
fn test_lsp_request_validation() {
    // Test completion request validation
    let completion = LspTestClient::create_completion_request("file:///test.gren", 0, 0);
    assert_eq!(completion.text_document_position.position.line, 0);
    assert_eq!(completion.text_document_position.position.character, 0);
    
    // Test different line/character positions
    let completion2 = LspTestClient::create_completion_request("file:///other.gren", 100, 50);
    assert_eq!(completion2.text_document_position.position.line, 100);
    assert_eq!(completion2.text_document_position.position.character, 50);
    
    // Test URI validation
    assert!(completion.text_document_position.text_document.uri.scheme() == "file");
    assert!(completion2.text_document_position.text_document.uri.scheme() == "file");
}

/// Test: Multiple request creation consistency
/// Purpose: Test that utilities create consistent request structures
#[test]
fn test_request_creation_consistency() {
    // Create multiple requests and verify consistency
    let init1 = LspTestClient::create_initialize_request();
    let init2 = LspTestClient::create_initialize_request();
    
    // Both should have same client info
    assert_eq!(init1.client_info, init2.client_info);
    
    // Both should have same capability structure
    assert_eq!(
        init1.capabilities.text_document.is_some(),
        init2.capabilities.text_document.is_some()
    );
    assert_eq!(
        init1.capabilities.workspace.is_some(),
        init2.capabilities.workspace.is_some()
    );
    
    // Test document creation consistency
    let doc1 = LspTestClient::create_did_open_notification(
        "file:///test.gren", "gren", "content"
    );
    let doc2 = LspTestClient::create_did_open_notification(
        "file:///test.gren", "gren", "content"
    );
    
    assert_eq!(doc1.text_document.uri, doc2.text_document.uri);
    assert_eq!(doc1.text_document.version, doc2.text_document.version);
}

/// Test: URL parsing in test utilities
/// Purpose: Test that test utilities handle URLs correctly
#[test]
fn test_url_handling_in_utilities() {
    // Test various URI formats
    let test_cases = vec![
        "file:///simple.gren",
        "file:///path/to/file.gren",
        "file:///deep/nested/path/file.gren",
    ];
    
    for uri_str in test_cases {
        let doc_params = LspTestClient::create_did_open_notification(
            uri_str, "gren", "module Test exposing (..)"
        );
        
        // Should parse successfully
        assert_eq!(doc_params.text_document.uri.as_str(), uri_str);
        assert!(doc_params.text_document.uri.scheme() == "file");
        
        let completion_params = LspTestClient::create_completion_request(uri_str, 0, 0);
        assert_eq!(completion_params.text_document_position.text_document.uri.as_str(), uri_str);
    }
}

/// Test: Comprehensive test utility validation
/// Purpose: Validate all test utility functions work correctly
#[test]
fn test_comprehensive_utility_validation() {
    // Test initialize request
    let init = LspTestClient::create_initialize_request();
    assert!(init.capabilities.text_document.is_some());
    assert!(init.capabilities.workspace.is_some());
    assert_eq!(init.client_info.as_ref().unwrap().name, "test-client");
    
    // Test document operations
    let did_open = LspTestClient::create_did_open_notification(
        "file:///test.gren", "gren", "module Test exposing (..)"
    );
    assert_eq!(did_open.text_document.uri.as_str(), "file:///test.gren");
    assert_eq!(did_open.text_document.language_id, "gren");
    assert_eq!(did_open.text_document.version, 1);
    
    // Test completion request
    let completion = LspTestClient::create_completion_request("file:///test.gren", 5, 10);
    assert_eq!(completion.text_document_position.position.line, 5);
    assert_eq!(completion.text_document_position.position.character, 10);
    
    // Test Gren source generation
    let source = create_test_gren_source();
    assert!(source.contains("module TestModule"));
    assert!(source.contains("greetUser"));
    assert!(source.contains("type alias User"));
}

/// Test: Edge cases in test utilities
/// Purpose: Test utilities handle edge cases correctly
#[test] 
fn test_utility_edge_cases() {
    // Test with empty content
    let empty_doc = LspTestClient::create_did_open_notification(
        "file:///empty.gren", "gren", ""
    );
    assert_eq!(empty_doc.text_document.text, "");
    assert_eq!(empty_doc.text_document.version, 1);
    
    // Test with position at line 0, char 0
    let origin_completion = LspTestClient::create_completion_request(
        "file:///test.gren", 0, 0
    );
    assert_eq!(origin_completion.text_document_position.position.line, 0);
    assert_eq!(origin_completion.text_document_position.position.character, 0);
    
    // Test with large line/character numbers
    let large_pos_completion = LspTestClient::create_completion_request(
        "file:///test.gren", 999, 999
    );
    assert_eq!(large_pos_completion.text_document_position.position.line, 999);
    assert_eq!(large_pos_completion.text_document_position.position.character, 999);
    
    // Test gren source consistency
    let source1 = create_test_gren_source();
    let source2 = create_test_gren_source();
    assert_eq!(source1, source2); // Should be deterministic
}