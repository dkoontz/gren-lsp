// Test with the actual test-files/Bytes.gren scenario
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

    println!("🔍 Testing real import scenario with test-files/Bytes.gren");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Read the actual test file
    let bytes_content = std::fs::read_to_string("/Users/david/dev/gren-lsp/test-files/Bytes.gren")?;

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

    // Create Result.gren file with hasValue function
    let result_content = r#"module Result exposing (..)

type Result error value = Ok value | Err error

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

    // Use the actual Bytes.gren content
    let bytes_uri = Url::parse("file:///test/Bytes.gren")?;
    let bytes_doc = TextDocumentItem {
        uri: bytes_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: bytes_content.clone(),
    };

    // Add all documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(maybe_doc)?;
        workspace.open_document(result_doc)?;
        workspace.open_document(bytes_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Find the line with "test = flatten hasValue"
    let mut test_line_num = None;
    let mut hasvalue_char_pos = None;

    for (line_num, line) in bytes_content.lines().enumerate() {
        if line.contains("test = flatten hasValue") {
            test_line_num = Some(line_num);
            hasvalue_char_pos = Some(line.find("hasValue").unwrap());
            println!("📋 Found test line {}: '{}'", line_num, line.trim());
            break;
        }
    }

    if let (Some(line_num), Some(char_pos)) = (test_line_num, hasvalue_char_pos) {
        // Test: Go to definition on "hasValue"
        println!(
            "\n🎯 Go-to-definition on 'hasValue' at line {}, character {}",
            line_num, char_pos
        );

        let goto_params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: bytes_uri.clone(),
                },
                position: Position {
                    line: line_num as u32,
                    character: char_pos as u32,
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
    } else {
        println!("❌ Could not find 'test = flatten hasValue' line in the file");
    }

    println!("\n🧪 Real import scenario test complete!");
    Ok(())
}
