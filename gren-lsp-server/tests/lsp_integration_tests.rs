use gren_lsp_server::test_utils::{create_test_gren_source, LspTestClient};
use lsp_types::*;

/// Basic tests for LSP test utilities
#[tokio::test]
async fn test_lsp_test_utilities() {
    let _test_client = LspTestClient::new();

    // Test initialize request creation
    let init_params = LspTestClient::create_initialize_request();
    assert!(init_params.capabilities.text_document.is_some());
    assert!(init_params.capabilities.workspace.is_some());
    assert_eq!(
        init_params.client_info.as_ref().unwrap().name,
        "test-client"
    );

    // Test document open notification creation
    let did_open_params = LspTestClient::create_did_open_notification(
        "file:///test.gren",
        "gren",
        create_test_gren_source(),
    );
    assert_eq!(
        did_open_params.text_document.uri.as_str(),
        "file:///test.gren"
    );
    assert_eq!(did_open_params.text_document.language_id, "gren");
    assert_eq!(did_open_params.text_document.version, 1);

    // Test completion request creation
    let completion_params = LspTestClient::create_completion_request("file:///test.gren", 5, 10);
    assert_eq!(completion_params.text_document_position.position.line, 5);
    assert_eq!(
        completion_params.text_document_position.position.character,
        10
    );

    // Test hover request creation
    let hover_params = LspTestClient::create_hover_request("file:///test.gren", 3, 8);
    assert_eq!(hover_params.text_document_position_params.position.line, 3);
    assert_eq!(
        hover_params
            .text_document_position_params
            .position
            .character,
        8
    );

    // Test goto definition request creation
    let goto_def_params =
        LspTestClient::create_goto_definition_request("file:///test.gren", 15, 10);
    assert_eq!(
        goto_def_params.text_document_position_params.position.line,
        15
    );
    assert_eq!(
        goto_def_params
            .text_document_position_params
            .position
            .character,
        10
    );
}

#[tokio::test]
async fn test_document_change_notification() {
    let _test_client = LspTestClient::new();

    // Test document change notification creation
    let changes = vec![TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 6,
            },
        }),
        range_length: Some(6),
        text: "module UpdatedModule".to_string(),
    }];

    let did_change_params =
        LspTestClient::create_did_change_notification("file:///test.gren", 2, changes);

    assert_eq!(
        did_change_params.text_document.uri.as_str(),
        "file:///test.gren"
    );
    assert_eq!(did_change_params.text_document.version, 2);
    assert_eq!(did_change_params.content_changes.len(), 1);
    assert_eq!(
        did_change_params.content_changes[0].text,
        "module UpdatedModule"
    );
}

#[tokio::test]
async fn test_gren_source_creation() {
    let source = create_test_gren_source();

    // Verify the test Gren source contains expected elements
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
