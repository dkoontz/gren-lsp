// Test client capabilities detection and hover format adaptation
use gren_lsp_core::Workspace;
use gren_lsp_protocol::handlers::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    println!("🔍 Testing client capabilities detection");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create a test file
    let test_content = r#"module Test exposing (add)

{-| A simple add function -}
add : Int -> Int -> Int
add x y = x + y
"#;

    let test_uri = Url::parse("file:///test/Test.gren")?;
    let test_doc = TextDocumentItem {
        uri: test_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: test_content.to_string(),
    };

    // Add document to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(test_doc)?;
    }

    println!("📁 Workspace setup complete");

    // Test 1: Client that supports markdown
    println!("\n🎯 Test 1: Client with markdown support");

    let markdown_client_caps = ClientCapabilities {
        text_document: Some(TextDocumentClientCapabilities {
            hover: Some(HoverClientCapabilities {
                content_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: test_uri.clone(),
            },
            position: Position {
                line: 3,      // Line with "add : Int -> Int -> Int"
                character: 0, // Position on "add"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers
        .hover_with_capabilities(hover_params.clone(), Some(&markdown_client_caps))
        .await
    {
        Ok(Some(hover)) => {
            if let HoverContents::Markup(content) = hover.contents {
                println!("✅ Markdown client - Content kind: {:?}", content.kind);
                println!(
                    "Content preview: {}",
                    content.value.lines().take(3).collect::<Vec<_>>().join(" ")
                );

                if content.kind == MarkupKind::Markdown {
                    println!("✅ Correctly returned markdown format");
                } else {
                    println!("❌ Expected markdown but got {:?}", content.kind);
                }

                if content.value.contains("**add**") {
                    println!("✅ Contains markdown formatting");
                } else {
                    println!("❌ Missing markdown formatting");
                }
            }
        }
        result => {
            println!("❌ Unexpected result: {:?}", result);
        }
    }

    // Test 2: Client that only supports plaintext
    println!("\n🎯 Test 2: Client with plaintext only");

    let plaintext_client_caps = ClientCapabilities {
        text_document: Some(TextDocumentClientCapabilities {
            hover: Some(HoverClientCapabilities {
                content_format: Some(vec![MarkupKind::PlainText]),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    match handlers
        .hover_with_capabilities(hover_params.clone(), Some(&plaintext_client_caps))
        .await
    {
        Ok(Some(hover)) => {
            if let HoverContents::Markup(content) = hover.contents {
                println!("✅ Plaintext client - Content kind: {:?}", content.kind);
                println!("Content: {}", content.value);

                if content.kind == MarkupKind::PlainText {
                    println!("✅ Correctly returned plaintext format");
                } else {
                    println!("❌ Expected plaintext but got {:?}", content.kind);
                }

                if !content.value.contains("**") && !content.value.contains("```") {
                    println!("✅ No markdown formatting in plaintext");
                } else {
                    println!("❌ Contains markdown formatting in plaintext");
                }
            }
        }
        result => {
            println!("❌ Unexpected result: {:?}", result);
        }
    }

    // Test 3: Client with no capabilities (backward compatibility)
    println!("\n🎯 Test 3: Client with no capabilities specified");

    match handlers.hover_with_capabilities(hover_params, None).await {
        Ok(Some(hover)) => {
            if let HoverContents::Markup(content) = hover.contents {
                println!("✅ No caps client - Content kind: {:?}", content.kind);

                if content.kind == MarkupKind::PlainText {
                    println!("✅ Defaults to plaintext when no capabilities");
                } else {
                    println!("❌ Expected plaintext default but got {:?}", content.kind);
                }
            }
        }
        result => {
            println!("❌ Unexpected result: {:?}", result);
        }
    }

    println!("\n🧪 Client capabilities test complete!");
    Ok(())
}
