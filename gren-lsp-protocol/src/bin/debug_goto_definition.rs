// Debug go-to-definition functionality with detailed logging
use gren_lsp_core::Workspace;
use gren_lsp_protocol::handlers::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to see debug output
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    println!("🔍 Debug go-to-definition functionality");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create Array.gren file
    let array_content = r#"module Array exposing (..)

flatten : Array (Array a) -> Array a
flatten array = 
    -- This is the local flatten function
    array
"#;

    let array_uri = Url::parse("file:///test/Array.gren")?;
    let array_doc = TextDocumentItem {
        uri: array_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: array_content.to_string(),
    };

    // Create Bytes.gren file
    let bytes_content = r#"module Bytes exposing (..)

flatten : Array Bytes -> Bytes
flatten array = 
    -- This is the Bytes module flatten function we want to find
    Bytes
"#;

    let bytes_uri = Url::parse("file:///test/Bytes.gren")?;
    let bytes_doc = TextDocumentItem {
        uri: bytes_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: bytes_content.to_string(),
    };

    // Add both documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(array_doc)?;
        workspace.open_document(bytes_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Test scenario: Go to definition on "flatten" in "Gren.Kernel.Bytes.flatten"
    // This simulates clicking on the "flatten" part of the qualified call
    println!("\n🎯 Testing go-to-definition on qualified call 'Gren.Kernel.Bytes.flatten'");

    // Let's test different character positions to see what happens
    let test_line = "    Gren.Kernel.Bytes.flatten";
    println!("Test line: '{}'", test_line);

    // Show character positions
    for (i, c) in test_line.chars().enumerate() {
        print!(
            "{}",
            if i % 10 == 0 {
                (i / 10).to_string().chars().next().unwrap_or(' ')
            } else {
                ' '
            }
        );
    }
    println!();
    for (i, _) in test_line.chars().enumerate() {
        print!("{}", i % 10);
    }
    println!();
    println!("{}", test_line);

    // Test different positions on the word "flatten"
    let flatten_start = test_line.find("flatten").unwrap();
    println!("'flatten' starts at position: {}", flatten_start);

    // Create a test document with this content
    let test_content = format!(
        r#"module Test exposing (..)

flatten : Array (Array a) -> Array a
flatten array = 
{}
"#,
        test_line
    );

    let test_uri = Url::parse("file:///test/Test.gren")?;
    let test_doc = TextDocumentItem {
        uri: test_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: test_content.to_string(),
    };

    {
        let mut workspace = workspace.write().await;
        workspace.open_document(test_doc)?;
    }

    // Test going to definition at different character positions on "flatten"
    let positions_to_test = vec![
        flatten_start,     // Start of "flatten"
        flatten_start + 3, // Middle of "flatten"
        flatten_start + 6, // End of "flatten"
    ];

    for (i, char_pos) in positions_to_test.iter().enumerate() {
        println!("\n--- Test {}: Character position {} ---", i + 1, char_pos);

        let goto_params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: test_uri.clone(),
                },
                position: Position {
                    line: 4, // Line with the qualified call
                    character: *char_pos as u32,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        println!(
            "Requesting go-to-definition at line 4, character {}",
            char_pos
        );

        match handlers.goto_definition(goto_params).await {
            Ok(Some(GotoDefinitionResponse::Scalar(location))) => {
                println!("✅ Found single definition:");
                println!("  File: {}", location.uri);
                println!(
                    "  Line: {}, Character: {}",
                    location.range.start.line, location.range.start.character
                );

                if location.uri.path().contains("Bytes.gren") {
                    println!("  ✅ CORRECT: Found definition in Bytes.gren");
                } else if location.uri.path().contains("Array.gren") {
                    println!(
                        "  ❌ INCORRECT: Found definition in Array.gren (should be Bytes.gren)"
                    );
                } else if location.uri.path().contains("Test.gren") {
                    println!("  ❌ INCORRECT: Found local definition in Test.gren (should be Bytes.gren)");
                } else {
                    println!("  ❓ Found definition in unexpected file");
                }
            }
            Ok(Some(GotoDefinitionResponse::Array(locations))) => {
                println!("✅ Found {} definitions:", locations.len());
                for (j, location) in locations.iter().enumerate() {
                    println!(
                        "  {}: {} (line {})",
                        j + 1,
                        location.uri,
                        location.range.start.line
                    );
                }
            }
            Ok(Some(GotoDefinitionResponse::Link(_links))) => {
                println!("✅ Found definition links");
            }
            Ok(None) => {
                println!("❌ No definition found");
            }
            Err(e) => {
                println!("❌ Error: {:?}", e);
            }
        }
    }

    println!("\n🧪 Debug test complete!");
    Ok(())
}
