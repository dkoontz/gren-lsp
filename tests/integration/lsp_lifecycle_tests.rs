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

    // Verify server still provides the same capabilities regardless of client
    assert!(result.capabilities.hover_provider.is_some());
    assert!(result.capabilities.completion_provider.is_some());

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_message_ordering() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Test that initialize must come first
    let result = client.initialize().await?;
    assert!(result.server_info.is_some());

    // Test that we can send multiple requests after initialization
    // (These will return None but shouldn't error)
    let _hover: Option<Hover> = client
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

    let _completion: Option<CompletionResponse> = client
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

    client.shutdown().await?;
    Ok(())
}