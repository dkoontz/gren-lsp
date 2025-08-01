use crate::helpers::lsp_test_client::LspTestClient;
use anyhow::Result;
use serde_json::{json, Value};

#[tokio::test]
async fn test_invalid_message_handling() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Initialize first
    client.initialize().await?;

    // Test sending malformed JSON-RPC
    // This should be handled gracefully by the server
    
    // Send a request with invalid method
    let response_result: Result<Value> = client
        .send_request_with_timeout("invalid/method", json!({}), 1000)
        .await;
    
    // Should get an error response, not a crash
    assert!(response_result.is_err());

    // Server should still be responsive after error
    let _hover: Option<tower_lsp::lsp_types::Hover> = client
        .send_request_with_timeout(
            "textDocument/hover",
            tower_lsp::lsp_types::HoverParams {
                text_document_position_params: tower_lsp::lsp_types::TextDocumentPositionParams {
                    text_document: tower_lsp::lsp_types::TextDocumentIdentifier {
                        uri: tower_lsp::lsp_types::Url::parse("file:///test.gren").unwrap(),
                    },
                    position: tower_lsp::lsp_types::Position::new(0, 0),
                },
                work_done_progress_params: tower_lsp::lsp_types::WorkDoneProgressParams::default(),
            },
            1000,
        )
        .await?;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_message_format_validation() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Test that the server responds with properly formatted messages
    let result = client.initialize().await?;

    // Verify the response has the expected JSON-RPC structure
    assert_eq!(result.server_info.as_ref().unwrap().name, "gren-lsp");
    
    // Verify capabilities are properly structured
    let caps = &result.capabilities;
    assert!(caps.text_document_sync.is_some());
    
    if let Some(tower_lsp::lsp_types::TextDocumentSyncCapability::Kind(sync_kind)) = &caps.text_document_sync {
        assert_eq!(*sync_kind, tower_lsp::lsp_types::TextDocumentSyncKind::INCREMENTAL);
    }

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_request_response_correlation() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    client.initialize().await?;

    // Send requests sequentially to avoid borrow checker issues
    let _hover_result: Option<tower_lsp::lsp_types::Hover> = client
        .send_request_with_timeout(
            "textDocument/hover",
            tower_lsp::lsp_types::HoverParams {
                text_document_position_params: tower_lsp::lsp_types::TextDocumentPositionParams {
                    text_document: tower_lsp::lsp_types::TextDocumentIdentifier {
                        uri: tower_lsp::lsp_types::Url::parse("file:///test1.gren").unwrap(),
                    },
                    position: tower_lsp::lsp_types::Position::new(0, 0),
                },
                work_done_progress_params: tower_lsp::lsp_types::WorkDoneProgressParams::default(),
            },
            1000,
        )
        .await?;

    let _completion_result: Option<tower_lsp::lsp_types::CompletionResponse> = client
        .send_request_with_timeout(
            "textDocument/completion",
            tower_lsp::lsp_types::CompletionParams {
                text_document_position: tower_lsp::lsp_types::TextDocumentPositionParams {
                    text_document: tower_lsp::lsp_types::TextDocumentIdentifier {
                        uri: tower_lsp::lsp_types::Url::parse("file:///test2.gren").unwrap(),
                    },
                    position: tower_lsp::lsp_types::Position::new(1, 5),
                },
                work_done_progress_params: tower_lsp::lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: tower_lsp::lsp_types::PartialResultParams::default(),
                context: None,
            },
            1000,
        )
        .await?;

    client.shutdown().await?;
    Ok(())
}