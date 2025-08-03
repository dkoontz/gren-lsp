use crate::helpers::lsp_test_client::LspTestClient;
use anyhow::Result;
use tower_lsp::lsp_types::*;

#[tokio::test]
async fn test_server_initialization() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Test initialization
    let result = client.initialize().await?;

    // Verify server info
    let server_info = result.server_info.expect("Server should provide info");
    assert_eq!(server_info.name, "gren-lsp");
    assert!(server_info.version.is_some());

    // Verify capabilities
    let caps = result.capabilities;
    assert!(caps.text_document_sync.is_some());
    assert!(caps.hover_provider.is_some());
    assert!(caps.completion_provider.is_some());
    assert!(caps.definition_provider.is_some());

    // Clean shutdown
    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_server_shutdown() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Initialize first
    client.initialize().await?;

    // Test shutdown sequence
    client.shutdown().await?;

    // Server should have exited cleanly (no assertion needed as shutdown() handles this)
    Ok(())
}

#[tokio::test]
async fn test_capability_negotiation() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Test with different client capabilities
    let params = InitializeParams {
        process_id: Some(std::process::id()),
        root_uri: Some(Url::parse("file:///tmp/test-workspace").unwrap()),
        capabilities: ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                hover: Some(HoverClientCapabilities {
                    dynamic_registration: Some(true),
                    content_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                }),
                completion: Some(CompletionClientCapabilities {
                    dynamic_registration: Some(true),
                    completion_item: Some(CompletionItemCapability {
                        documentation_format: Some(vec![MarkupKind::Markdown]),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    let result: InitializeResult = client
        .send_request_with_timeout("initialize", params, 1000)
        .await?;

    // Send initialized notification
    client.send_notification("initialized", InitializedParams {}).await?;

    // CAPABILITY INTERSECTION VALIDATION: Server MUST only advertise supported capabilities
    
    // Client declared support for hover and completion - server MUST provide them
    assert!(
        result.capabilities.hover_provider.is_some(),
        "Server MUST advertise hover when client supports it"
    );
    assert!(
        result.capabilities.completion_provider.is_some(),
        "Server MUST advertise completion when client supports it"
    );
    assert!(
        result.capabilities.definition_provider.is_some(),
        "Server MUST advertise definition when client supports it"
    );
    
    // PRECISE CAPABILITY VALIDATION: Test client that does NOT support hover
    let no_hover_params = InitializeParams {
        process_id: Some(std::process::id()),
        root_uri: Some(Url::parse("file:///tmp/test-workspace").unwrap()),
        capabilities: ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                // Explicitly NO hover capability
                hover: None,
                completion: Some(CompletionClientCapabilities {
                    dynamic_registration: Some(false),
                    ..Default::default()
                }),
                definition: Some(GotoCapability {
                    dynamic_registration: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    
    // Test true capability negotiation with different client
    let mut client2 = LspTestClient::spawn().await?;
    let result2: InitializeResult = client2
        .send_request_with_timeout("initialize", no_hover_params, 1000)
        .await?;
    
    client2.send_notification("initialized", InitializedParams {}).await?;
    
    // DETERMINISTIC CAPABILITY INTERSECTION: Server MUST NOT advertise unsupported capabilities
    assert!(
        result2.capabilities.hover_provider.is_none(),
        "Server MUST NOT advertise hover when client does not support it"
    );
    assert!(
        result2.capabilities.completion_provider.is_some(),
        "Server MUST advertise completion when client supports it"
    );
    assert!(
        result2.capabilities.definition_provider.is_some(),
        "Server MUST advertise definition when client supports it"
    );
    
    // Test minimal client with no text document capabilities
    let minimal_params = InitializeParams {
        process_id: Some(std::process::id()),
        root_uri: Some(Url::parse("file:///tmp/test-workspace").unwrap()),
        capabilities: ClientCapabilities {
            text_document: None, // No text document capabilities at all
            ..Default::default()
        },
        ..Default::default()
    };
    
    let mut client3 = LspTestClient::spawn().await?;
    let result3: InitializeResult = client3
        .send_request_with_timeout("initialize", minimal_params, 1000)
        .await?;
    
    client3.send_notification("initialized", InitializedParams {}).await?;
    
    // Server MUST NOT advertise any text document capabilities
    assert!(
        result3.capabilities.hover_provider.is_none(),
        "Server MUST NOT advertise hover when client has no text document capabilities"
    );
    assert!(
        result3.capabilities.completion_provider.is_none(),
        "Server MUST NOT advertise completion when client has no text document capabilities"
    );
    assert!(
        result3.capabilities.definition_provider.is_none(),
        "Server MUST NOT advertise definition when client has no text document capabilities"
    );
    
    client2.shutdown().await?;
    client3.shutdown().await?;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_message_ordering() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Test that initialize must come first
    let result = client.initialize().await?;
    assert!(result.server_info.is_some());

    // Test that requests to unopened documents return specific error or None (deterministic behavior)
    let hover_result: Option<Hover> = client
        .send_request_with_timeout(
            "textDocument/hover",
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse("file:///test.gren").unwrap(),
                    },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
            1000,
        )
        .await?;
    
    // For unopened documents, hover should return None (deterministic behavior)
    assert!(hover_result.is_none(), "Hover on unopened document should return None");

    let completion_result: Option<CompletionResponse> = client
        .send_request_with_timeout(
            "textDocument/completion",
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse("file:///test.gren").unwrap(),
                    },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            },
            1000,
        )
        .await?;
    
    // For unopened documents, completion should return None (deterministic behavior)
    assert!(completion_result.is_none(), "Completion on unopened document should return None");

    client.shutdown().await?;
    Ok(())
}