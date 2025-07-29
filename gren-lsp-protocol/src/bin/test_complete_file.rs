// Test complete file structure like the user's issue
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing complete file structure like user's issue");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // More complete version with both module and type
    let test_content = r#"module Bytes exposing
  ( Bytes
  , empty
  , isEmpty
  , length
  )

{-| A sequence of bytes -}
type Bytes = Bytes

empty : Bytes
empty = Bytes

isEmpty : Bytes -> Bool
isEmpty (Bytes) = True

length : Bytes -> Int
length (Bytes) = 0"#;

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

    // Test raw symbol extraction first
    {
        let workspace = workspace.read().await;
        let raw_symbols = workspace.get_file_symbols(&doc_uri)?;
        println!("📋 Raw extracted symbols ({}):", raw_symbols.len());
        for (i, symbol) in raw_symbols.iter().enumerate() {
            println!(
                "  {}: '{}' ({:?}) container: {:?}",
                i + 1,
                symbol.name,
                symbol.kind,
                symbol.container_name
            );
        }
    }

    // Test document symbols
    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: doc_uri.clone(),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.document_symbols(params).await {
        Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
            println!("\n✅ Final document symbols ({}): ", symbols.len());
            for (i, symbol) in symbols.iter().enumerate() {
                println!("  {}. {} ({:?})", i + 1, symbol.name, symbol.kind);
                if let Some(children) = &symbol.children {
                    for (j, child) in children.iter().enumerate() {
                        println!("    {}.{}: {} ({:?})", i + 1, j + 1, child.name, child.kind);
                    }
                }
            }

            // Analyze results
            let modules = symbols
                .iter()
                .filter(|s| s.kind == SymbolKind::MODULE)
                .count();
            let types = symbols
                .iter()
                .filter(|s| s.kind == SymbolKind::CLASS)
                .count();
            let functions = symbols
                .iter()
                .filter(|s| s.kind == SymbolKind::FUNCTION)
                .count();

            println!("\n📊 Summary:");
            println!("  Modules: {}", modules);
            println!("  Types: {}", types);
            println!("  Functions: {}", functions);

            if modules == 1 && types >= 1 && functions >= 3 {
                println!("✅ All expected symbols present!");
            } else {
                println!("❌ Missing some expected symbols");
            }
        }
        _ => println!("❌ No symbols or error"),
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
