// Test the real Bytes.gren scenario
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

    println!("🔍 Testing real Bytes.gren scenario");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create the real Bytes.gren content (simplified)
    let bytes_content = r#"module Bytes exposing (..)

import Gren.Kernel.Bytes

{-| Flatten all `Bytes` in an `Array` into a single `Bytes`.
-}
flatten : Array Bytes -> Bytes
flatten =
  Gren.Kernel.Bytes.flatten

things = [1,2,3]

test = flatten toString

foo = 123
"#;

    let bytes_uri = Url::parse("file:///test/Bytes.gren")?;
    let bytes_doc = TextDocumentItem {
        uri: bytes_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: bytes_content.to_string(),
    };

    // Add document to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(bytes_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Show the content with line numbers for reference
    println!("\n📄 File content:");
    for (i, line) in bytes_content.lines().enumerate() {
        println!("{:2}: {}", i, line);
    }

    // Test 1: Go to definition on "flatten" in "Gren.Kernel.Bytes.flatten" (line 8)
    println!("\n🎯 Test 1: Go-to-definition on 'flatten' in 'Gren.Kernel.Bytes.flatten' (line 8)");

    let flatten_line = "  Gren.Kernel.Bytes.flatten";
    let flatten_pos = flatten_line.find("flatten").unwrap();
    println!("Character position of 'flatten': {}", flatten_pos);

    let goto_params1 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 8,                       // Line with "Gren.Kernel.Bytes.flatten"
                character: flatten_pos as u32, // Position on "flatten"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params1).await {
        Ok(Some(GotoDefinitionResponse::Scalar(location))) => {
            println!("❌ UNEXPECTED: Found definition (should be none for Gren.Kernel.*):");
            println!("  File: {}", location.uri);
            println!(
                "  Line: {}, Character: {}",
                location.range.start.line, location.range.start.character
            );

            if location.range.start.line == 6 {
                println!("  ❌ INCORRECT: This points to the local flatten function definition");
            }
        }
        Ok(Some(GotoDefinitionResponse::Array(locations))) => {
            println!("❌ UNEXPECTED: Found {} definitions", locations.len());
            for location in &locations {
                println!("  - {} (line {})", location.uri, location.range.start.line);
            }
        }
        Ok(Some(GotoDefinitionResponse::Link(_links))) => {
            println!("❌ UNEXPECTED: Found definition links (should be none for Gren.Kernel.*)");
        }
        Ok(None) => {
            println!(
                "✅ CORRECT: No definition found for Gren.Kernel.Bytes.flatten (it's built-in)"
            );
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 2: Go to definition on local "flatten" function name (line 6)
    println!("\n🎯 Test 2: Go-to-definition on local 'flatten' function definition (line 6)");

    let goto_params2 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 6,      // Line with "flatten : Array Bytes -> Bytes"
                character: 0, // Position on "flatten"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params2).await {
        Ok(Some(_response)) => {
            println!("✅ Found definition for local flatten (this is expected)");
        }
        Ok(None) => {
            println!("ℹ️  No definition found for local flatten (this is acceptable)");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 3: Go to definition on "flatten" in usage "test = flatten toString" (line 12)
    println!("\n🎯 Test 3: Go-to-definition on 'flatten' in 'test = flatten toString' (line 12)");

    let goto_params3 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 12,     // Line with "test = flatten toString"
                character: 7, // Position on "flatten"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params3).await {
        Ok(Some(GotoDefinitionResponse::Scalar(location))) => {
            println!("✅ Found definition:");
            println!("  File: {}", location.uri);
            println!(
                "  Line: {}, Character: {}",
                location.range.start.line, location.range.start.character
            );

            if location.range.start.line == 6 {
                println!("  ✅ CORRECT: Points to the local flatten function definition (line 6)");
            } else {
                println!("  ❓ Unexpected line number");
            }
        }
        Ok(Some(GotoDefinitionResponse::Array(locations))) => {
            println!("✅ Found {} definitions", locations.len());
            for location in &locations {
                println!("  - {} (line {})", location.uri, location.range.start.line);
            }
        }
        Ok(Some(GotoDefinitionResponse::Link(_links))) => {
            println!("✅ Found definition links");
        }
        Ok(None) => {
            println!("❌ No definition found for local flatten usage");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Real scenario test complete!");
    Ok(())
}
