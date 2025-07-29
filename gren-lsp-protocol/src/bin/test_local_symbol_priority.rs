// Test that local symbols are prioritized and no workspace fallback occurs
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

    println!("🔍 Testing local symbol priority and no workspace fallback");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create Array.gren file with flatten function (should NOT be returned for local calls)
    let array_content = r#"module Array exposing (..)

flatten : Array (Array a) -> Array a
flatten array = 
    -- This is Array.flatten - should NOT be returned for local calls
    array
"#;

    let array_uri = Url::parse("file:///test/Array.gren")?;
    let array_doc = TextDocumentItem {
        uri: array_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: array_content.to_string(),
    };

    // Create Maybe.gren file with hasValue function
    let maybe_content = r#"module Maybe exposing (..)

type Maybe a = Just a | Nothing

hasValue : Maybe a -> Bool
hasValue maybe =
    case maybe of
        Just _ -> True
        Nothing -> False
"#;

    let maybe_uri = Url::parse("file:///test/Maybe.gren")?;
    let maybe_doc = TextDocumentItem {
        uri: maybe_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: maybe_content.to_string(),
    };

    // Create Bytes.gren file with local flatten and imported hasValue
    let bytes_content = r#"module Bytes exposing (..)

import Array exposing (Array)
import Basics exposing (..)
import Maybe exposing (Maybe, hasValue)
import String exposing (String)
import Task exposing (Task)
import Gren.Kernel.Bytes

flatten : Array Bytes -> Bytes
flatten =
  Gren.Kernel.Bytes.flatten

things = [1,2,3]

test = flatten hasValue

foo = 123
"#;

    let bytes_uri = Url::parse("file:///test/Bytes.gren")?;
    let bytes_doc = TextDocumentItem {
        uri: bytes_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: bytes_content.to_string(),
    };

    // Add all documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(array_doc)?;
        workspace.open_document(maybe_doc)?;
        workspace.open_document(bytes_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Test 1: Go to definition on LOCAL "flatten" in "test = flatten hasValue"
    println!(
        "\n🎯 Test 1: Go-to-definition on LOCAL 'flatten' (should find ONLY local definition)"
    );

    let goto_params1 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 15,     // Line with "test = flatten hasValue"
                character: 7, // Position on "flatten"
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

            if location.uri.path().contains("Bytes.gren") {
                println!("  ✅ CORRECT: Found local flatten definition in Bytes.gren");
            } else if location.uri.path().contains("Array.gren") {
                println!("  ❌ INCORRECT: Found Array.flatten (should only find local)");
            } else {
                println!("  ❓ Unexpected file");
            }
        }
        Ok(Some(GotoDefinitionResponse::Array(locations))) => {
            println!("❌ Found multiple definitions (should be single local only):");
            for (i, location) in locations.iter().enumerate() {
                println!(
                    "  {}: {} (line {})",
                    i + 1,
                    location.uri,
                    location.range.start.line
                );

                if location.uri.path().contains("Bytes.gren") {
                    println!("    ✅ This one is correct (local)");
                } else if location.uri.path().contains("Array.gren") {
                    println!("    ❌ This one is incorrect (should not include Array.flatten)");
                }
            }
        }
        Ok(Some(GotoDefinitionResponse::Link(_links))) => {
            println!("✅ Found definition links");
        }
        Ok(None) => {
            println!("❌ No definition found for local flatten");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 2: Go to definition on IMPORTED "hasValue" in "test = flatten hasValue"
    println!("\n🎯 Test 2: Go-to-definition on IMPORTED 'hasValue' (should find ONLY Maybe.gren)");

    let goto_params2 = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 15,      // Line with "test = flatten hasValue"
                character: 15, // Position on "hasValue"
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

            if location.uri.path().contains("Maybe.gren") {
                println!("  ✅ CORRECT: Found imported hasValue in Maybe.gren");
            } else {
                println!("  ❓ Unexpected file");
            }
        }
        Ok(Some(GotoDefinitionResponse::Array(locations))) => {
            println!("❌ Found multiple definitions (should be single import only):");
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
            println!("✅ Found definition links");
        }
        Ok(None) => {
            println!("❌ No definition found for imported hasValue");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Local symbol priority test complete!");
    Ok(())
}
