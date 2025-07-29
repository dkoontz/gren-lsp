// Test go-to-definition functionality
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing go-to-definition functionality");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create multiple files to test cross-file go-to-definition

    // File 1: Utils.gren - contains definitions
    let utils_content = r#"module Utils exposing (..)

type Result error value = Ok value | Err error

isEmpty : String -> Bool
isEmpty str = str == ""

length : String -> Int
length str = String.length str

map : (a -> b) -> List a -> List b
map fn list = []
"#;

    let utils_uri = Url::parse("file:///test/Utils.gren")?;
    let utils_doc = TextDocumentItem {
        uri: utils_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: utils_content.to_string(),
    };

    // File 2: Main.gren - contains usage of definitions
    let main_content = r#"module Main exposing (..)

import Utils

main : String
main = 
    let
        result = Utils.isEmpty "hello"
        len = Utils.length "world"
    in
        if result then "empty" else "not empty"
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
        workspace.open_document(utils_doc)?;
        workspace.open_document(main_doc)?;
    }

    // Test 1: Go to definition of "isEmpty" in Main.gren
    println!("\n🎯 Test 1: Go to definition of 'isEmpty'");
    let goto_params1 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 7,       // Line with "Utils.isEmpty"
                character: 23, // Position on "isEmpty"
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
            }
        }
        Ok(Some(GotoDefinitionResponse::Link(_links))) => {
            println!("✅ Found definition links (advanced feature)");
        }
        Ok(None) => {
            println!("❌ No definition found");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 2: Go to definition of "length" in Main.gren
    println!("\n🎯 Test 2: Go to definition of 'length'");
    let goto_params2 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 8,       // Line with "Utils.length"
                character: 21, // Position on "length"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params2).await {
        Ok(Some(GotoDefinitionResponse::Scalar(location))) => {
            println!("✅ Found single definition:");
            println!("  File: {}", location.uri);
            println!(
                "  Line: {}, Character: {}",
                location.range.start.line, location.range.start.character
            );
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
            }
        }
        Ok(Some(GotoDefinitionResponse::Link(_links))) => {
            println!("✅ Found definition links (advanced feature)");
        }
        Ok(None) => {
            println!("❌ No definition found");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 3: Go to definition of local variable "result"
    println!("\n🎯 Test 3: Go to definition of local variable 'result'");
    let goto_params3 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 10,      // Line with "if result then"
                character: 11, // Position on "result"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.goto_definition(goto_params3).await {
        Ok(Some(response)) => {
            println!("✅ Found definition for local variable (this might not work yet due to scope limitations)");
        }
        Ok(None) => {
            println!("ℹ️  No definition found for local variable (expected - local scope not fully implemented)");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
