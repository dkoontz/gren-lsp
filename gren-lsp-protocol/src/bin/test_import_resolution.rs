// Test import-aware symbol resolution
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

    println!("🔍 Testing import-aware symbol resolution");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create Maybe.gren file with hasValue function
    let maybe_content = r#"module Maybe exposing (..)

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

    // Create Result.gren file with hasValue function
    let result_content = r#"module Result exposing (..)

hasValue : Result error value -> Bool
hasValue result =
    case result of
        Ok _ -> True
        Err _ -> False
"#;

    let result_uri = Url::parse("file:///test/Result.gren")?;
    let result_doc = TextDocumentItem {
        uri: result_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: result_content.to_string(),
    };

    // Create Bytes.gren file that imports hasValue from Maybe specifically
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
        workspace.open_document(maybe_doc)?;
        workspace.open_document(result_doc)?;
        workspace.open_document(bytes_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Show the import we're testing
    println!("\n📋 Testing import: 'import Maybe exposing (Maybe, hasValue)'");
    println!("Usage: 'test = flatten hasValue' (line 15)");

    // Test: Go to definition on "hasValue" in "test = flatten hasValue"
    println!("\n🎯 Go-to-definition on 'hasValue' (should resolve to Maybe.gren)");

    let goto_params = GotoDefinitionParams {
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

    match handlers.goto_definition(goto_params).await {
        Ok(Some(GotoDefinitionResponse::Scalar(location))) => {
            println!("✅ Found single definition:");
            println!("  File: {}", location.uri);
            println!(
                "  Line: {}, Character: {}",
                location.range.start.line, location.range.start.character
            );

            if location.uri.path().contains("Maybe.gren") {
                println!("  ✅ CORRECT: Resolved to Maybe.gren based on import");
            } else if location.uri.path().contains("Result.gren") {
                println!("  ❌ INCORRECT: Resolved to Result.gren (should be Maybe.gren based on import)");
            } else {
                println!("  ❓ Resolved to unexpected file");
            }
        }
        Ok(Some(GotoDefinitionResponse::Array(locations))) => {
            println!("❌ Found multiple definitions (should be single based on import):");
            for (i, location) in locations.iter().enumerate() {
                println!(
                    "  {}: {} (line {})",
                    i + 1,
                    location.uri,
                    location.range.start.line
                );

                if location.uri.path().contains("Maybe.gren") {
                    println!("    ✅ This one is correct (imported)");
                } else if location.uri.path().contains("Result.gren") {
                    println!("    ❌ This one is incorrect (not imported)");
                }
            }
        }
        Ok(Some(GotoDefinitionResponse::Link(_links))) => {
            println!("✅ Found definition links");
        }
        Ok(None) => {
            println!("❌ No definition found for hasValue");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Import resolution test complete!");
    Ok(())
}
