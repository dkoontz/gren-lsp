// Debug the specific Endianness duplication issue
use gren_lsp_core::Workspace;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Debugging Endianness duplication issue");
    
    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    
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
    
    // Get raw symbols to see what's being extracted
    let workspace = workspace.read().await;
    let symbols = workspace.get_file_symbols(&doc_uri)?;
    
    println!("📋 Raw extracted symbols ({}): ", symbols.len());
    for (i, symbol) in symbols.iter().enumerate() {
        println!("  {}: '{}' ({:?}) at line {} (range: {:?})", 
            i + 1, 
            symbol.name, 
            symbol.kind,
            symbol.location.range.start.line + 1,
            symbol.location.range
        );
        if let Some(sig) = &symbol.type_signature {
            println!("      Type: {}", sig);
        }
    }
    
    println!("\n💡 The duplication is happening in symbol extraction, not conversion");
    println!("💡 We need to fix the tree-sitter queries to avoid duplicate symbols");
    
    Ok(())
}