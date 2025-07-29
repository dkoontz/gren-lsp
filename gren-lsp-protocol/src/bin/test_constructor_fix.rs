// Test the constructor association fix
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing constructor association fix");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Test with both types that were getting mixed up
    let test_content = r#"type Bytes = Bytes

-- many lines of code would be here

type Endianness = LE | BE"#;

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

    // Test document symbols with fixed constructor logic
    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: doc_uri.clone(),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    match handlers.document_symbols(params).await {
        Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
            println!(
                "✅ Document symbols after constructor fix ({}): ",
                symbols.len()
            );
            for (i, symbol) in symbols.iter().enumerate() {
                println!("  {}. {} ({:?})", i + 1, symbol.name, symbol.kind);
                if let Some(children) = &symbol.children {
                    for (j, child) in children.iter().enumerate() {
                        println!("    {}.{}: {} ({:?})", i + 1, j + 1, child.name, child.kind);
                    }
                }
            }

            // Check for proper separation
            let bytes_symbol = symbols.iter().find(|s| s.name == "Bytes");
            let endianness_symbol = symbols.iter().find(|s| s.name == "Endianness");

            match (bytes_symbol, endianness_symbol) {
                (Some(bytes), Some(endianness)) => {
                    println!("\n🔍 Checking constructor associations:");

                    // Bytes should have no children or just the Bytes constructor
                    let bytes_children = bytes.children.as_ref().map(|c| c.len()).unwrap_or(0);
                    println!("  Bytes has {} children", bytes_children);

                    // Endianness should have LE and BE as children
                    let endianness_children =
                        endianness.children.as_ref().map(|c| c.len()).unwrap_or(0);
                    println!("  Endianness has {} children", endianness_children);

                    if let Some(children) = &endianness.children {
                        let has_le = children.iter().any(|c| c.name == "LE");
                        let has_be = children.iter().any(|c| c.name == "BE");
                        let has_bytes = children.iter().any(|c| c.name == "Bytes");

                        if has_le && has_be && !has_bytes {
                            println!("✅ Constructor association is correct!");
                        } else {
                            println!("❌ Constructor association still has issues:");
                            println!("    LE: {}, BE: {}, Bytes: {}", has_le, has_be, has_bytes);
                        }
                    }
                }
                _ => println!("❌ Could not find both Bytes and Endianness symbols"),
            }
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
