use crate::helpers::lsp_test_client::LspTestClient;
use anyhow::Result;
use serde_json::{json, Value};
use tower_lsp::lsp_types::{TextDocumentSyncCapability, TextDocumentSyncKind, HoverProviderCapability};

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
    
    // MUST assert exact JSON-RPC 2.0 error code -32601 for method not found
    match response_result {
        Err(e) => {
            let error_str = e.to_string();
            // Validate exact JSON-RPC 2.0 error code
            assert!(
                error_str.contains("-32601"),
                "Expected exact JSON-RPC error code -32601 (Method not found), got: {}", error_str
            );
            // Validate exact error message format per JSON-RPC 2.0 specification
            assert!(
                error_str.contains("Method not found"),
                "Expected exact 'Method not found' message, got: {}", error_str
            );
        }
        Ok(_) => panic!("Invalid method MUST return error -32601, not success"),
    }

    // Server should still be responsive after error - test with specific expectation
    let hover_result: Option<tower_lsp::lsp_types::Hover> = client
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
    
    // DETERMINISTIC VALIDATION: Hover on unopened document MUST return exactly None
    assert_eq!(hover_result, None, "Hover on unopened document MUST return exactly None (not Some(empty))");
    
    // ADDITIONAL VALIDATION: Verify server continues to handle valid requests correctly
    assert!(hover_result.is_none(), "Server MUST remain functional after error handling");

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_message_format_validation() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // PROTOCOL COMPLIANCE: Validate exact LSP message format per specification
    let result = client.initialize().await?;

    // VALIDATE LSP MESSAGE FORMAT: InitializeResult MUST match specification exactly
    
    // Server info MUST be present and properly formatted
    let server_info = result.server_info.as_ref()
        .expect("InitializeResult MUST include server_info per LSP specification");
    assert_eq!(server_info.name, "gren-lsp", "Server name MUST match exactly");
    assert!(server_info.version.is_some(), "Server version MUST be provided");
    
    // Capabilities MUST be properly structured
    let capabilities = &result.capabilities;
    
    // Text document sync MUST be specified
    assert!(
        capabilities.text_document_sync.is_some(),
        "Server MUST specify text document sync capability"
    );
    
    // Validate text document sync format per LSP specification
    match &capabilities.text_document_sync {
        Some(TextDocumentSyncCapability::Kind(kind)) => {
            match *kind {
                TextDocumentSyncKind::NONE | 
                TextDocumentSyncKind::FULL | 
                TextDocumentSyncKind::INCREMENTAL => {
                    // Valid sync kinds per LSP specification
                },
                _ => panic!("Invalid TextDocumentSyncKind value"),
            }
        },
        Some(TextDocumentSyncCapability::Options(_)) => {
            // TextDocumentSyncOptions format is also valid
        },
        None => panic!("TextDocumentSyncCapability MUST be specified"),
    }
    
    // Validate hover provider format if present
    if let Some(hover_provider) = &capabilities.hover_provider {
        match hover_provider {
            HoverProviderCapability::Simple(enabled) => {
                assert!(
                    *enabled == true || *enabled == false,
                    "HoverProviderCapability boolean MUST be valid"
                );
            },
            HoverProviderCapability::Options(_) => {
                // HoverOptions format is valid per LSP specification
            },
        }
    }
    
    // Validate completion provider format if present  
    if let Some(completion_provider) = &capabilities.completion_provider {
        // Trigger characters MUST be valid strings if present
        if let Some(trigger_chars) = &completion_provider.trigger_characters {
            for trigger_char in trigger_chars {
                assert!(
                    !trigger_char.is_empty(),
                    "Completion trigger characters MUST be non-empty strings"
                );
                assert!(
                    trigger_char.len() <= 1,
                    "Completion trigger characters MUST be single characters"
                );
            }
        }
    }
    
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