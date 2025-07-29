// Test hover with real test-files/Bytes.gren scenario
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

    println!("🔍 Testing hover with real test-files/Bytes.gren scenario");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Read the actual test file
    let bytes_content = std::fs::read_to_string("/Users/david/dev/gren-lsp/test-files/Bytes.gren")?;

    // Create Maybe.gren file for hasValue
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

    // Use the actual Bytes.gren content
    let bytes_uri = Url::parse("file:///test/Bytes.gren")?;
    let bytes_doc = TextDocumentItem {
        uri: bytes_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: bytes_content.clone(),
    };

    // Add documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(maybe_doc)?;
        workspace.open_document(bytes_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Find the line with "test = flatten hasValue"
    let mut test_line_num = None;
    let mut flatten_char_pos = None;
    let mut hasvalue_char_pos = None;

    for (line_num, line) in bytes_content.lines().enumerate() {
        if line.contains("test = flatten hasValue") {
            test_line_num = Some(line_num);
            flatten_char_pos = Some(line.find("flatten").unwrap());
            hasvalue_char_pos = Some(line.find("hasValue").unwrap());
            println!("📋 Found test line {}: '{}'", line_num, line.trim());
            break;
        }
    }

    if let (Some(line_num), Some(flatten_pos), Some(hasvalue_pos)) =
        (test_line_num, flatten_char_pos, hasvalue_char_pos)
    {
        // Test 1: Hover on "flatten"
        println!(
            "\n🎯 Test 1: Hover on 'flatten' at line {}, character {}",
            line_num, flatten_pos
        );

        let hover_params1 = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: bytes_uri.clone(),
                },
                position: Position {
                    line: line_num as u32,
                    character: flatten_pos as u32,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        match handlers.hover(hover_params1).await {
            Ok(Some(hover)) => {
                println!("✅ Found hover content for flatten:");
                if let HoverContents::Markup(content) = hover.contents {
                    println!("---");
                    println!("{}", content.value);
                    println!("---");
                }
            }
            Ok(None) => {
                println!("❌ No hover content found for flatten");
            }
            Err(e) => {
                println!("❌ Error: {:?}", e);
            }
        }

        // Test 2: Hover on "hasValue"
        println!(
            "\n🎯 Test 2: Hover on 'hasValue' at line {}, character {}",
            line_num, hasvalue_pos
        );

        let hover_params2 = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: bytes_uri.clone(),
                },
                position: Position {
                    line: line_num as u32,
                    character: hasvalue_pos as u32,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        match handlers.hover(hover_params2).await {
            Ok(Some(hover)) => {
                println!("✅ Found hover content for hasValue:");
                if let HoverContents::Markup(content) = hover.contents {
                    println!("---");
                    println!("{}", content.value);
                    println!("---");
                }
            }
            Ok(None) => {
                println!("❌ No hover content found for hasValue");
            }
            Err(e) => {
                println!("❌ Error: {:?}", e);
            }
        }

        // Test 3: Find a function with documentation in the real file
        println!("\n🎯 Test 3: Looking for functions with documentation in Bytes.gren");

        // Look for a function definition with doc comments
        let mut doc_function_line = None;
        let mut doc_function_name = None;

        for (line_num, line) in bytes_content.lines().enumerate() {
            let line = line.trim();
            if line.ends_with(" : ") || (line.contains(" : ") && line.contains(" -> ")) {
                // This looks like a function signature
                if let Some(name_end) = line.find(" :") {
                    let name = &line[..name_end].trim();
                    if !name.is_empty() && !name.contains("=") {
                        doc_function_line = Some(line_num);
                        doc_function_name = Some(name.to_string());
                        println!("Found function '{}' at line {}: '{}'", name, line_num, line);
                        break;
                    }
                }
            }
        }

        if let (Some(line_num), Some(function_name)) = (doc_function_line, doc_function_name) {
            let hover_params3 = HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: bytes_uri.clone(),
                    },
                    position: Position {
                        line: line_num as u32,
                        character: 0, // Start of line
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            };

            match handlers.hover(hover_params3).await {
                Ok(Some(hover)) => {
                    println!("✅ Found hover content for '{}':", function_name);
                    if let HoverContents::Markup(content) = hover.contents {
                        println!("---");
                        println!("{}", content.value);
                        println!("---");
                    }
                }
                Ok(None) => {
                    println!("❌ No hover content found for '{}'", function_name);
                }
                Err(e) => {
                    println!("❌ Error: {:?}", e);
                }
            }
        }
    } else {
        println!("❌ Could not find 'test = flatten hasValue' line in the file");
    }

    println!("\n🧪 Real scenario hover test complete!");
    Ok(())
}
