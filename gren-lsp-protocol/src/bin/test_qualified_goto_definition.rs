// Test qualified go-to-definition functionality
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing qualified go-to-definition functionality");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create multiple files to test qualified go-to-definition

    // File 1: Bytes.gren - contains the function we want to navigate to
    let bytes_content = r#"module Bytes exposing (..)

type Bytes = Bytes

flatten : Array Bytes -> Bytes
flatten array = 
    -- This is the REAL flatten function we want to find
    Bytes
"#;

    let bytes_uri = Url::parse("file:///test/Bytes.gren")?;
    let bytes_doc = TextDocumentItem {
        uri: bytes_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: bytes_content.to_string(),
    };

    // File 2: Main.gren - contains the problematic case
    let main_content = r#"module Main exposing (..)

flatten : Array Bytes -> Bytes
flatten =
    -- This is calling the Bytes module flatten, NOT the local one above
    Gren.Kernel.Bytes.flatten

example : String -> String  
example str =
    let
        result = String.length str
        other = Utils.isEmpty str
    in
        str
"#;

    let main_uri = Url::parse("file:///test/Main.gren")?;
    let main_doc = TextDocumentItem {
        uri: main_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: main_content.to_string(),
    };

    // Add both documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(bytes_doc)?;
        workspace.open_document(main_doc)?;
    }

    // Test 1: Go to definition of "flatten" in qualified call "Gren.Kernel.Bytes.flatten"
    println!("\n🎯 Test 1: Go to definition of 'flatten' in 'Gren.Kernel.Bytes.flatten'");
    let goto_params1 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 5,       // Line with "Gren.Kernel.Bytes.flatten"
                character: 22, // Position on "flatten" part
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params1).await {
        Ok(Some(GotoDefinitionResponse::Scalar(location))) => {
            println!("✅ Found single definition:");
            println!("  File: {}", location.uri);
            println!(
                "  Line: {}, Character: {}",
                location.range.start.line, location.range.start.character
            );

            // Check if this is the correct flatten (from Bytes.gren, not Main.gren)
            if location.uri.path().contains("Bytes.gren") {
                println!("  ✅ CORRECT: Found definition in Bytes.gren");
            } else if location.uri.path().contains("Main.gren") {
                println!("  ❌ INCORRECT: Found definition in Main.gren (should be Bytes.gren)");
            } else {
                println!("  ℹ️  Found definition in unexpected file");
            }
        }
        Ok(Some(GotoDefinitionResponse::Array(locations))) => {
            println!("✅ Found {} definitions:", locations.len());
            for (i, location) in locations.iter().enumerate() {
                println!(
                    "  {}: {} (line {})",
                    i + 1,
                    location.uri,
                    location.range.start.line
                );

                if location.uri.path().contains("Bytes.gren") {
                    println!("    ✅ This one is correct (Bytes.gren)");
                } else if location.uri.path().contains("Main.gren") {
                    println!("    ❌ This one is incorrect (Main.gren)");
                }
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

    // Test 2: Go to definition of unqualified "flatten" (should find local one)
    println!("\n🎯 Test 2: Go to definition of local 'flatten' function");
    let goto_params2 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 2,      // Line with function definition "flatten : Array Bytes -> Bytes"
                character: 0, // Position on "flatten"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params2).await {
        Ok(Some(response)) => {
            println!("✅ Found definition for local flatten");
        }
        Ok(None) => {
            println!("ℹ️  No definition found for local flatten (expected)");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 3: Go to definition of "length" in "String.length"
    println!("\n🎯 Test 3: Go to definition of 'length' in 'String.length'");
    let goto_params3 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 10,      // Line with "String.length str"
                character: 22, // Position on "length"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params3).await {
        Ok(Some(_response)) => {
            println!("✅ Found definition for String.length");
        }
        Ok(None) => {
            println!("ℹ️  No definition found for String.length (expected for built-in)");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
