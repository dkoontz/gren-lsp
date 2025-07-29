// Test basic code completion functionality
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing basic code completion");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create a test file with some functions and types
    let test_content = r#"module Test exposing (..)

type Color = Red | Green | Blue

isEmpty : String -> Bool
isEmpty str = str == ""

length : String -> Int  
length str = String.length str

-- User is typing here and wants completion
myFunction x = 
"#;

    let doc_uri = Url::parse("file:///test/Test.gren")?;
    let doc_item = TextDocumentItem {
        uri: doc_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: test_content.to_string(),
    };

    // Add document to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(doc_item)?;
    }

    // Test completion at the end of the file (after "myFunction x = ")
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: doc_uri.clone(),
            },
            position: Position {
                line: 11,     // Last line where user is typing
                character: 0, // Beginning of line after the incomplete function
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };

    println!("🔍 Requesting completion at line 11, character 0");

    match handlers.completion(completion_params).await {
        Ok(Some(CompletionResponse::Array(items))) => {
            println!("✅ Found {} completion items:", items.len());

            // Group by kind for better display
            let mut functions = Vec::new();
            let mut types = Vec::new();
            let mut keywords = Vec::new();
            let mut others = Vec::new();

            for item in &items {
                match item.kind {
                    Some(CompletionItemKind::FUNCTION) => functions.push(item),
                    Some(CompletionItemKind::CLASS | CompletionItemKind::CONSTRUCTOR) => {
                        types.push(item)
                    }
                    Some(CompletionItemKind::KEYWORD) => keywords.push(item),
                    _ => others.push(item),
                }
            }

            if !functions.is_empty() {
                println!("\n📋 Functions:");
                for func in functions.iter().take(5) {
                    println!(
                        "  - {} {}",
                        func.label,
                        func.detail
                            .as_ref()
                            .map(|d| format!("({})", d))
                            .unwrap_or_default()
                    );
                }
            }

            if !types.is_empty() {
                println!("\n🏷️  Types:");
                for typ in types.iter().take(5) {
                    println!(
                        "  - {} {}",
                        typ.label,
                        typ.detail
                            .as_ref()
                            .map(|d| format!("({})", d))
                            .unwrap_or_default()
                    );
                }
            }

            if !keywords.is_empty() {
                println!("\n🔤 Keywords:");
                for keyword in keywords.iter().take(5) {
                    println!("  - {}", keyword.label);
                }
            }

            if !others.is_empty() {
                println!("\n📦 Others:");
                for other in others.iter().take(5) {
                    println!("  - {} ({:?})", other.label, other.kind);
                }
            }

            // Test that we have expected completions
            let has_isempty = items.iter().any(|i| i.label == "isEmpty");
            let has_length = items.iter().any(|i| i.label == "length");
            let has_color = items.iter().any(|i| i.label == "Color");
            let has_if_keyword = items.iter().any(|i| i.label == "if");

            println!("\n🎯 Expected completions:");
            println!(
                "  isEmpty function: {}",
                if has_isempty { "✅" } else { "❌" }
            );
            println!(
                "  length function: {}",
                if has_length { "✅" } else { "❌" }
            );
            println!("  Color type: {}", if has_color { "✅" } else { "❌" });
            println!(
                "  'if' keyword: {}",
                if has_if_keyword { "✅" } else { "❌" }
            );

            if has_isempty && has_length && has_color && has_if_keyword {
                println!("\n🎉 Basic completion is working!");
            } else {
                println!("\n⚠️  Some expected completions are missing");
            }
        }
        Ok(Some(CompletionResponse::List(_))) => {
            println!("✅ Got completion list (not tested in this example)");
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
