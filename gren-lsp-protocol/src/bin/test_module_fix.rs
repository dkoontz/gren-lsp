// Test the module duplication fix
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing module duplication fix");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Test with the problematic module declaration from the user
    let test_content = r#"module Bytes exposing
  ( Bytes
  , empty
  , isEmpty
  , length
  )

type Bytes = Bytes

empty : Bytes
empty = Bytes

isEmpty : Bytes -> Bool
isEmpty (Bytes) = True"#;

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

    // Test document symbols with fixed module logic
    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: doc_uri.clone(),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.document_symbols(params).await {
        Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
            println!("✅ Document symbols after module fix ({}): ", symbols.len());
            for (i, symbol) in symbols.iter().enumerate() {
                println!("  {}. {} ({:?})", i + 1, symbol.name, symbol.kind);
                if let Some(children) = &symbol.children {
                    for (j, child) in children.iter().enumerate() {
                        println!("    {}.{}: {} ({:?})", i + 1, j + 1, child.name, child.kind);
                    }
                }
            }

            // Check for duplicate modules
            let module_symbols: Vec<_> = symbols
                .iter()
                .filter(|s| s.kind == SymbolKind::MODULE)
                .collect();
            println!("\n🔍 Found {} module symbols:", module_symbols.len());
            for module in &module_symbols {
                println!("  - '{}'", module.name);
            }

            if module_symbols.len() == 1 {
                println!("✅ No duplicate modules - fix working!");
            } else {
                println!("❌ Still has {} module entries", module_symbols.len());
            }

            // Check that we have the right symbols
            let has_bytes_module = symbols
                .iter()
                .any(|s| s.name == "Bytes" && s.kind == SymbolKind::MODULE);
            let has_bytes_type = symbols
                .iter()
                .any(|s| s.name == "Bytes" && s.kind == SymbolKind::CLASS);
            let has_empty_function = symbols
                .iter()
                .any(|s| s.name == "empty" && s.kind == SymbolKind::FUNCTION);

            println!("\n🎯 Symbol check:");
            println!("  Bytes module: {}", has_bytes_module);
            println!("  Bytes type: {}", has_bytes_type);
            println!("  empty function: {}", has_empty_function);
        }
        Ok(Some(DocumentSymbolResponse::Flat(symbols))) => {
            println!("✅ Found {} flat symbols", symbols.len());
        }
        Ok(None) => {
            println!("❌ No symbols returned");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
