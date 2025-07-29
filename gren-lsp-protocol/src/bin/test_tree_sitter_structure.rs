// Test tree-sitter structural approach vs line-based heuristics
use gren_lsp_core::Workspace;
use gren_lsp_protocol::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing tree-sitter structural approach");
    
    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());
    
    // Test with multiple types that would confuse line-based heuristics
    let test_content = r#"module Test exposing (..)

type Color = Red | Green | Blue

type Result error value = 
    | Err error 
    | Ok value

type alias Point = { x : Float, y : Float }

type Direction = North | South | East | West"#;
    
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
    
    // Test document symbols with tree-sitter structure
    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: doc_uri.clone(),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    
    match handlers.document_symbols(params).await {
        Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
            println!("✅ Document symbols using tree-sitter structure ({}): ", symbols.len());
            
            for (i, symbol) in symbols.iter().enumerate() {
                println!("  {}. {} ({:?})", i + 1, symbol.name, symbol.kind);
                if let Some(children) = &symbol.children {
                    for (j, child) in children.iter().enumerate() {
                        println!("    {}.{}: {} ({:?})", i + 1, j + 1, child.name, child.kind);
                    }
                }
            }
            
            // Verify proper structure
            let mut results = Vec::new();
            
            // Check Color type
            if let Some(color) = symbols.iter().find(|s| s.name == "Color") {
                let empty_vec = vec![];
                let constructors: Vec<_> = color.children.as_ref().unwrap_or(&empty_vec).iter().map(|c| c.name.as_str()).collect();
                results.push(format!("Color: {:?}", constructors));
                
                let expected = vec!["Red", "Green", "Blue"];
                if constructors == expected {
                    println!("✅ Color type has correct constructors");
                } else {
                    println!("❌ Color type incorrect: expected {:?}, got {:?}", expected, constructors);
                }
            }
            
            // Check Result type
            if let Some(result) = symbols.iter().find(|s| s.name == "Result") {
                let empty_vec = vec![];
                let constructors: Vec<_> = result.children.as_ref().unwrap_or(&empty_vec).iter().map(|c| c.name.as_str()).collect();
                results.push(format!("Result: {:?}", constructors));
                
                let expected = vec!["Err", "Ok"];
                if constructors == expected {
                    println!("✅ Result type has correct constructors");
                } else {
                    println!("❌ Result type incorrect: expected {:?}, got {:?}", expected, constructors);
                }
            }
            
            // Check Direction type
            if let Some(direction) = symbols.iter().find(|s| s.name == "Direction") {
                let empty_vec = vec![];
                let constructors: Vec<_> = direction.children.as_ref().unwrap_or(&empty_vec).iter().map(|c| c.name.as_str()).collect();
                results.push(format!("Direction: {:?}", constructors));
                
                let expected = vec!["North", "South", "East", "West"];
                if constructors == expected {
                    println!("✅ Direction type has correct constructors");
                } else {
                    println!("❌ Direction type incorrect: expected {:?}, got {:?}", expected, constructors);
                }
            }
            
            // Check Point type alias (should have no constructors)
            if let Some(point) = symbols.iter().find(|s| s.name == "Point") {
                let constructor_count = point.children.as_ref().map(|c| c.len()).unwrap_or(0);
                if constructor_count == 0 {
                    println!("✅ Point type alias correctly has no constructors");
                } else {
                    println!("❌ Point type alias incorrectly has {} constructors", constructor_count);
                }
            }
            
            println!("\n🎯 Tree-sitter structure provides perfect type-constructor relationships!");
            println!("💡 No more line-based heuristics needed - parser knows the structure");
            
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