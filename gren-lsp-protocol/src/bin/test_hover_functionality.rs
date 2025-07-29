// Test hover functionality
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

    println!("🔍 Testing hover functionality");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create Maybe.gren file with hasValue function and documentation
    let maybe_content = r#"module Maybe exposing (..)

type Maybe a = Just a | Nothing

{-| Check if a Maybe value contains a value.

This function returns True if the Maybe is `Just something`,
and False if it's `Nothing`.

    hasValue (Just 42) == True
    hasValue Nothing == False
-}
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

    // Create Bytes.gren file that imports and uses hasValue
    let bytes_content = r#"module Bytes exposing (..)

import Array exposing (Array)
import Basics exposing (..)
import Maybe exposing (Maybe, hasValue)
import String exposing (String)
import Task exposing (Task)
import Gren.Kernel.Bytes

{-| Flatten all `Bytes` in an `Array` into a single `Bytes`.
-}
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

    // Add documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(maybe_doc)?;
        workspace.open_document(bytes_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Test 1: Hover on local function "flatten"
    println!("\n🎯 Test 1: Hover on LOCAL function 'flatten'");

    let hover_params1 = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 17,     // Line with "test = flatten hasValue"
                character: 7, // Position on "flatten"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.hover(hover_params1).await {
        Ok(Some(hover)) => {
            println!("✅ Found hover content for local flatten:");
            if let HoverContents::Markup(content) = hover.contents {
                println!("---");
                println!("{}", content.value);
                println!("---");

                // Verify it contains expected information
                if content.value.contains("**flatten**") && content.value.contains("function") {
                    println!("✅ Contains function name and type");
                } else {
                    println!("❌ Missing function name or type");
                }

                if content.value.contains("Array Bytes -> Bytes") {
                    println!("✅ Contains type signature");
                } else {
                    println!("❌ Missing type signature");
                }

                if content.value.contains("Bytes.gren") {
                    println!("✅ Contains file location");
                } else {
                    println!("❌ Missing file location");
                }
            }
        }
        Ok(None) => {
            println!("❌ No hover content found for local flatten");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 2: Hover on imported function "hasValue"
    println!("\n🎯 Test 2: Hover on IMPORTED function 'hasValue'");

    let hover_params2 = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 17,      // Line with "test = flatten hasValue"
                character: 15, // Position on "hasValue"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.hover(hover_params2).await {
        Ok(Some(hover)) => {
            println!("✅ Found hover content for imported hasValue:");
            if let HoverContents::Markup(content) = hover.contents {
                println!("---");
                println!("{}", content.value);
                println!("---");

                // Verify it contains expected information
                if content.value.contains("**hasValue**") && content.value.contains("function") {
                    println!("✅ Contains function name and type");
                } else {
                    println!("❌ Missing function name or type");
                }

                if content.value.contains("Maybe a -> Bool") {
                    println!("✅ Contains type signature");
                } else {
                    println!("❌ Missing type signature");
                }

                if content
                    .value
                    .contains("Check if a Maybe value contains a value")
                {
                    println!("✅ Contains documentation");
                } else {
                    println!("❌ Missing documentation");
                }

                if content.value.contains("Maybe.gren") {
                    println!("✅ Contains correct file location");
                } else {
                    println!("❌ Missing or incorrect file location");
                }
            }
        }
        Ok(None) => {
            println!("❌ No hover content found for imported hasValue");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 3: Hover on builtin "Gren.Kernel.Bytes.flatten"
    println!("\n🎯 Test 3: Hover on BUILTIN 'Gren.Kernel.Bytes.flatten'");

    let hover_params3 = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: bytes_uri.clone(),
            },
            position: Position {
                line: 13,      // Line with "Gren.Kernel.Bytes.flatten"
                character: 26, // Position on "flatten" part
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.hover(hover_params3).await {
        Ok(Some(hover)) => {
            println!("❌ UNEXPECTED: Found hover content for builtin (should be None)");
            if let HoverContents::Markup(content) = hover.contents {
                println!("Content: {}", content.value);
            }
        }
        Ok(None) => {
            println!("✅ CORRECT: No hover content for builtin function");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Hover functionality test complete!");
    Ok(())
}
