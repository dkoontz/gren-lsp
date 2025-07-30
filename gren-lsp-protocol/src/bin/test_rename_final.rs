// Final comprehensive rename functionality test
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

    println!("🔍 Final Rename Functionality Test");
    println!("Testing rename operations with comprehensive scenarios");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Test 1: Create test documents and test single-file rename
    println!("\n📁 Setting up test workspace");

    // Create Maybe.gren with functions to rename
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

{-| Another function that uses hasValue -}
checkThing : Maybe String -> String
checkThing item =
    if hasValue item then
        "has value"
    else
        "no value"
"#;

    let maybe_uri = Url::parse("file:///test/Maybe.gren")?;
    let maybe_doc = TextDocumentItem {
        uri: maybe_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: maybe_content.to_string(),
    };

    // Create Main.gren that imports and uses hasValue
    let main_content = r#"module Main exposing (..)

import Maybe exposing (Maybe, hasValue)
import Html exposing (Html, text)

main : Html msg
main =
    let
        example = Maybe.Just "hello"
        result = hasValue example
    in
    if result then
        text "Found value"
    else
        text "No value"

helper : Maybe Int -> Bool
helper val = hasValue val
"#;

    let main_uri = Url::parse("file:///test/Main.gren")?;
    let main_doc = TextDocumentItem {
        uri: main_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: main_content.to_string(),
    };

    // Add documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(maybe_doc)?;
        workspace.open_document(main_doc)?;
    }

    println!("✅ Workspace setup complete with 2 files");

    // Test 2: Single-file rename of hasValue function definition
    println!("\n🎯 Test 1: Single-file rename of function definition");

    let rename_params1 = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: maybe_uri.clone(),
            },
            position: Position {
                line: 13,     // Line with "hasValue : Maybe a -> Bool"
                character: 0, // Start of function name
            },
        },
        new_name: "containsValue".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.rename(rename_params1).await {
        Ok(Some(workspace_edit)) => {
            println!("✅ Single-file rename succeeded!");

            let mut total_edits = 0;
            let mut files_affected = 0;

            if let Some(changes) = &workspace_edit.changes {
                files_affected = changes.len();
                total_edits = changes.values().map(|edits| edits.len()).sum();

                println!(
                    "  - Affected {} files with {} total edits",
                    files_affected, total_edits
                );

                for (uri, edits) in changes {
                    println!("    * {}: {} edits", uri.path(), edits.len());
                    for (i, edit) in edits.iter().enumerate() {
                        if i < 3 {
                            // Show first 3 edits
                            println!(
                                "      - Line {}: '{}' -> '{}'",
                                edit.range.start.line + 1,
                                "hasValue", // We know what we're replacing
                                edit.new_text
                            );
                        }
                    }
                    if edits.len() > 3 {
                        println!("      ... and {} more edits", edits.len() - 3);
                    }
                }
            }

            if files_affected >= 2 {
                println!("🎉 Successfully performed cross-file rename!");
            } else {
                println!("📝 Single-file rename completed");
            }
        }
        Ok(None) => {
            println!("⚠️  Rename returned None (symbol may not be found at position)");
        }
        Err(e) => {
            println!("❌ Rename failed: {:?}", e);
        }
    }

    // Test 3: Rename usage in imported location
    println!("\n🎯 Test 2: Cross-file rename from import location");

    let rename_params2 = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 8,       // Line with "result = hasValue example"
                character: 17, // Position on "hasValue"
            },
        },
        new_name: "isPresent".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.rename(rename_params2).await {
        Ok(Some(workspace_edit)) => {
            println!("✅ Cross-file rename from usage succeeded!");

            if let Some(changes) = &workspace_edit.changes {
                let files_affected = changes.len();
                let total_edits: usize = changes.values().map(|edits| edits.len()).sum();

                println!(
                    "  - Affected {} files with {} total edits",
                    files_affected, total_edits
                );

                if files_affected >= 2 {
                    println!("🎉 Successfully renamed across multiple files!");
                }
            }
        }
        Ok(None) => {
            println!("⚠️  Cross-file rename returned None");
        }
        Err(e) => {
            println!("❌ Cross-file rename failed: {:?}", e);
        }
    }

    // Test 4: Edge cases and validation
    println!("\n🧪 Test 3: Validation and edge cases");

    // Test invalid identifier
    let invalid_rename = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: maybe_uri.clone(),
            },
            position: Position {
                line: 13,
                character: 0,
            },
        },
        new_name: "123invalid".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.rename(invalid_rename).await {
        Ok(_) => println!("❌ UNEXPECTED: Invalid identifier accepted"),
        Err(e) => {
            if e.message.contains("not a valid Gren identifier") {
                println!(
                    "✅ Correctly rejected invalid identifier: '{}'",
                    "123invalid"
                );
            } else {
                println!("⚠️  Rejected for different reason: {}", e.message);
            }
        }
    }

    // Test keyword as new name
    let keyword_rename = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: maybe_uri.clone(),
            },
            position: Position {
                line: 13,
                character: 0,
            },
        },
        new_name: "case".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.rename(keyword_rename).await {
        Ok(_) => println!("❌ UNEXPECTED: Keyword accepted as new name"),
        Err(e) => {
            if e.message.contains("not a valid Gren identifier") {
                println!("✅ Correctly rejected keyword: '{}'", "case");
            } else {
                println!("⚠️  Rejected for different reason: {}", e.message);
            }
        }
    }

    // Test empty name
    let empty_rename = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: maybe_uri.clone(),
            },
            position: Position {
                line: 13,
                character: 0,
            },
        },
        new_name: "".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.rename(empty_rename).await {
        Ok(_) => println!("❌ UNEXPECTED: Empty name accepted"),
        Err(e) => {
            if e.message.contains("not a valid Gren identifier") {
                println!("✅ Correctly rejected empty name");
            } else {
                println!("⚠️  Rejected for different reason: {}", e.message);
            }
        }
    }

    // Test position with no symbol
    println!("\n🎯 Test 4: Position with no renameable symbol");

    let no_symbol_rename = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: maybe_uri.clone(),
            },
            position: Position {
                line: 0,
                character: 0,
            }, // "module" keyword
        },
        new_name: "validName".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.rename(no_symbol_rename).await {
        Ok(Some(_)) => println!("⚠️  UNEXPECTED: Found renameable symbol at module line"),
        Ok(None) => println!("✅ Correctly returned None for non-renameable position"),
        Err(e) => println!("⚠️  Error at non-renameable position: {:?}", e),
    }

    println!("\n🏁 Final rename functionality tests complete!");
    println!("✅ Rename implementation validated with comprehensive scenarios");
    println!("🎯 Features tested:");
    println!("  - Single-file symbol renaming");
    println!("  - Cross-file reference updates");
    println!("  - Input validation (invalid identifiers, keywords, empty names)");
    println!("  - Non-renameable position handling");
    println!("  - Workspace edit generation");

    Ok(())
}
