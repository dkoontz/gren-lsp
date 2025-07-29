// Test workspace-wide completion functionality
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing workspace-wide completion");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create multiple files to test workspace completion

    // File 1: Utils.gren
    let utils_content = r#"module Utils exposing (..)

type Result error value = Ok value | Err error

map : (a -> b) -> List a -> List b
map fn list = []

filter : (a -> Bool) -> List a -> List a  
filter predicate list = []
"#;

    let utils_uri = Url::parse("file:///test/Utils.gren")?;
    let utils_doc = TextDocumentItem {
        uri: utils_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: utils_content.to_string(),
    };

    // File 2: Main.gren (where user is typing)
    let main_content = r#"module Main exposing (..)

import Utils

main : String
main = 
    -- User typing here, should get completion from Utils.gren and keywords
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

    // Test completion in Main.gren
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 6, // Line with comment where user is typing
                character: 10,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };

    println!("🔍 Requesting completion in Main.gren");

    match handlers.completion(completion_params).await {
        Ok(Some(CompletionResponse::Array(items))) => {
            println!("✅ Found {} completion items:", items.len());

            // Categorize completions by source
            let mut local_symbols = Vec::new();
            let mut workspace_symbols = Vec::new();
            let mut keywords = Vec::new();

            for item in &items {
                if let Some(sort_text) = &item.sort_text {
                    if sort_text.starts_with("0_") {
                        local_symbols.push(item);
                    } else if sort_text.starts_with("1_") {
                        workspace_symbols.push(item);
                    } else if sort_text.starts_with("2_") {
                        keywords.push(item);
                    }
                } else {
                    // Default to local if no sort_text
                    local_symbols.push(item);
                }
            }

            println!(
                "\n📋 Local symbols (from Main.gren): {}",
                local_symbols.len()
            );
            for symbol in local_symbols.iter().take(5) {
                println!(
                    "  - {} {}",
                    symbol.label,
                    symbol
                        .detail
                        .as_ref()
                        .map(|d| format!("({})", d))
                        .unwrap_or_default()
                );
            }

            println!(
                "\n🌐 Workspace symbols (from other files): {}",
                workspace_symbols.len()
            );
            for symbol in workspace_symbols.iter().take(5) {
                println!(
                    "  - {} {}",
                    symbol.label,
                    symbol
                        .detail
                        .as_ref()
                        .map(|d| format!("({})", d))
                        .unwrap_or_default()
                );
            }

            println!("\n🔤 Keywords: {}", keywords.len());
            for keyword in keywords.iter().take(5) {
                println!("  - {}", keyword.label);
            }

            // Check for expected symbols from Utils.gren
            let has_map = items.iter().any(|i| i.label == "map");
            let has_filter = items.iter().any(|i| i.label == "filter");
            let has_result = items.iter().any(|i| i.label == "Result");
            let has_main = items.iter().any(|i| i.label == "main");

            println!("\n🎯 Expected completions:");
            println!("  map (from Utils): {}", if has_map { "✅" } else { "❌" });
            println!(
                "  filter (from Utils): {}",
                if has_filter { "✅" } else { "❌" }
            );
            println!(
                "  Result (from Utils): {}",
                if has_result { "✅" } else { "❌" }
            );
            println!("  main (local): {}", if has_main { "✅" } else { "❌" });

            if has_map && has_filter && has_result {
                println!("\n🎉 Workspace completion is working!");
            } else {
                println!("\n⚠️  Some workspace symbols are missing");
            }
        }
        Ok(Some(CompletionResponse::List(_))) => {
            println!("✅ Got completion list (advanced features)");
        }
        Ok(None) => {
            println!("❌ No completion items returned");
        }
        Err(e) => {
            println!("❌ Completion error: {:?}", e);
        }
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
