// Test handling of Gren.Kernel.* built-in modules
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

    println!("🔍 Testing Gren.Kernel.* built-in module handling");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create Array.gren file with local flatten
    let array_content = r#"module Array exposing (..)

flatten : Array (Array a) -> Array a
flatten array = 
    -- This is a local flatten function that should NOT be matched
    -- when someone calls Gren.Kernel.Bytes.flatten
    Gren.Kernel.Bytes.flatten array
"#;

    let array_uri = Url::parse("file:///test/Array.gren")?;
    let array_doc = TextDocumentItem {
        uri: array_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: array_content.to_string(),
    };

    // Add document to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(array_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Test: Go to definition on "flatten" in "Gren.Kernel.Bytes.flatten"
    println!("\n🎯 Testing go-to-definition on Gren.Kernel.Bytes.flatten");

    let goto_params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: array_uri.clone(),
            },
            position: Position {
                line: 5,       // Line with "Gren.Kernel.Bytes.flatten array"
                character: 26, // Position on "flatten" part of qualified call
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params).await {
        Ok(Some(GotoDefinitionResponse::Scalar(location))) => {
            println!("❌ UNEXPECTED: Found definition (should be none for built-in):");
            println!("  File: {}", location.uri);
            println!(
                "  Line: {}, Character: {}",
                location.range.start.line, location.range.start.character
            );

            if location.uri.path().contains("Array.gren") {
                println!("  ❌ INCORRECT: This points to the local flatten, not the built-in one");
            }
        }
        Ok(Some(GotoDefinitionResponse::Array(locations))) => {
            println!(
                "❌ UNEXPECTED: Found {} definitions (should be none for built-in)",
                locations.len()
            );
            for (i, location) in locations.iter().enumerate() {
                println!(
                    "  {}: {} (line {})",
                    i + 1,
                    location.uri,
                    location.range.start.line
                );
            }
        }
        Ok(Some(GotoDefinitionResponse::Link(_links))) => {
            println!("❌ UNEXPECTED: Found definition links (should be none for built-in)");
        }
        Ok(None) => {
            println!(
                "✅ CORRECT: No definition found for Gren.Kernel.Bytes.flatten (it's a built-in)"
            );
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test: Go to definition on local flatten function
    println!("\n🎯 Testing go-to-definition on local flatten function");

    let goto_params2 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: array_uri.clone(),
            },
            position: Position {
                line: 2,      // Line with local "flatten : Array (Array a) -> Array a"
                character: 0, // Position on "flatten"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params2).await {
        Ok(Some(_response)) => {
            println!("✅ Found definition for local flatten function (expected)");
        }
        Ok(None) => {
            println!("ℹ️  No definition found for local flatten (this is acceptable)");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Kernel module test complete!");
    Ok(())
}
