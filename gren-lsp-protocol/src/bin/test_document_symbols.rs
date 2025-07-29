// Test document symbols functionality that VS Code calls
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing document symbols (VS Code Outline panel functionality)");

    // Create workspace and handlers
    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create test content similar to Bytes.gren
    let test_content = r#"module Bytes exposing (..)

{-| A sequence of bytes -}
type Bytes = Bytes

{-| Get the length of bytes -}
length : Bytes -> Int
length (Bytes) = 0

{-| Convert to string -}
toString : Bytes -> String  
toString (Bytes) = ""

type alias Config = { size : Int }

decode : String -> Maybe Bytes
decode str = Nothing
"#;

    let doc_uri = Url::parse("file:///test/Bytes.gren")?;
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

    println!("✅ Document added to workspace");

    // This is the exact call VS Code makes for the Outline panel
    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: doc_uri.clone(),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    println!("🔍 Calling document_symbols (like VS Code Outline panel)...");
    match handlers.document_symbols(params).await {
        Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
            println!("✅ SUCCESS: Found {} document symbols", symbols.len());
            println!("🎉 This should show up in VS Code Outline panel!");

            for (i, symbol) in symbols.iter().enumerate() {
                println!(
                    "  {}. {} ({:?}) at line {}",
                    i + 1,
                    symbol.name,
                    symbol.kind,
                    symbol.range.start.line + 1
                );
                if let Some(detail) = &symbol.detail {
                    println!("      Type: {}", detail);
                }
                if let Some(children) = &symbol.children {
                    println!("      Children: {}", children.len());
                    for child in children.iter().take(3) {
                        println!("        - {} ({:?})", child.name, child.kind);
                    }
                }
            }
        }
        Ok(Some(DocumentSymbolResponse::Flat(symbols))) => {
            println!(
                "✅ Found {} flat symbols (should work in VS Code)",
                symbols.len()
            );
            for symbol in symbols.iter().take(10) {
                println!("  - {} ({:?})", symbol.name, symbol.kind);
            }
        }
        Ok(None) => {
            println!("❌ PROBLEM: No symbols returned!");
            println!("❓ This is exactly what VS Code is experiencing");
            println!("🔧 Let's debug why this happens...");

            // Debug the workspace state
            let workspace = workspace.read().await;
            match workspace.get_file_symbols(&doc_uri) {
                Ok(raw_symbols) => {
                    println!("✅ Raw workspace has {} symbols", raw_symbols.len());
                    for symbol in raw_symbols.iter().take(5) {
                        println!("  - {} ({:?})", symbol.name, symbol.kind);
                    }
                    println!("💡 Symbols exist but document_symbols() returns None");
                }
                Err(e) => {
                    println!("❌ Workspace get_file_symbols failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Error in document_symbols: {:?}", e);
        }
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
