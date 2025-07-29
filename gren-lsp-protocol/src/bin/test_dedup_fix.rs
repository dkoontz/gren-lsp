// Test the document symbol deduplication fix
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing fixed deduplication logic with Endianness example");
    
    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());
    
    // Test with the problematic type definition
    let test_content = "type Endianness = LE | BE";
    
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
    
    // Test document symbols with new deduplication logic
    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: doc_uri.clone(),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    
    match handlers.document_symbols(params).await {
        Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
            println!("✅ Document symbols after deduplication ({}): ", symbols.len());
            for (i, symbol) in symbols.iter().enumerate() {
                println!("  {}. {} ({:?})", i + 1, symbol.name, symbol.kind);
                if let Some(children) = &symbol.children {
                    for (j, child) in children.iter().enumerate() {
                        println!("    {}.{}: {} ({:?})", i + 1, j + 1, child.name, child.kind);
                    }
                }
            }
            
            // Check if we fixed the duplication
            let has_duplicate_endianness = symbols.iter().filter(|s| s.name == "Endianness").count() > 1;
            if has_duplicate_endianness {
                println!("❌ Still has duplicate Endianness entries");
            } else {
                println!("✅ No duplicate Endianness entries - deduplication working!");
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