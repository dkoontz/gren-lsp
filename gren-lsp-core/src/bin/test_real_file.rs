// Test symbol extraction with a real Gren file
use gren_lsp_core::Workspace;
use lsp_types::*;
use std::fs;

fn main() -> anyhow::Result<()> {
    
    println!("🔍 Testing symbol extraction with real Gren file");
    
    // Try to find Bytes.gren file
    let possible_paths = [
        "Bytes.gren",
        "../Bytes.gren", 
        "../../Bytes.gren",
        "/Users/david/dev/gren-lsp/Bytes.gren",
        "/Users/david/Bytes.gren",
    ];
    
    let mut file_content = None;
    let mut file_path = None;
    
    for path in &possible_paths {
        if let Ok(content) = fs::read_to_string(path) {
            file_content = Some(content);
            file_path = Some(path.to_string());
            break;
        }
    }
    
    let (content, path) = match (file_content, file_path) {
        (Some(content), Some(path)) => {
            println!("✅ Found Gren file: {}", path);
            println!("📄 File size: {} bytes", content.len());
            (content, path)
        }
        _ => {
            println!("❌ Could not find Bytes.gren file");
            println!("💡 Please run this from the directory containing your Gren file, or specify the correct path");
            println!("\n🧪 Using test content instead:");
            let test_content = r#"
module Bytes exposing (..)

{-| A sequence of bytes -}
type Bytes = Bytes

{-| Get the length of bytes -}
length : Bytes -> Int
length (Bytes) = 0

{-| Convert to string -}
toString : Bytes -> String  
toString (Bytes) = ""
"#;
            (test_content.to_string(), "test_content".to_string())
        }
    };
    
    // Show first few lines
    let lines: Vec<&str> = content.lines().take(10).collect();
    println!("\n📋 First {} lines:", lines.len());
    for (i, line) in lines.iter().enumerate() {
        println!("  {:2}: {}", i + 1, line);
    }
    if content.lines().count() > 10 {
        println!("  ... ({} more lines)", content.lines().count() - 10);
    }
    
    // Test symbol extraction
    println!("\n🔧 Testing symbol extraction...");
    
    let mut workspace = Workspace::new()?;
    let doc_uri = Url::parse(&format!("file:///{}", path.replace(" ", "%20")))?;
    let doc_item = TextDocumentItem {
        uri: doc_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: content,
    };
    
    // Add document to workspace
    workspace.open_document(doc_item)?;
    
    // Get symbols for the file
    let symbols = workspace.get_file_symbols(&doc_uri)?;
    
    if symbols.is_empty() {
        println!("❌ No symbols found!");
        println!("💡 This might indicate:");
        println!("   - The tree-sitter parser failed");
        println!("   - The queries don't match the file structure");
        println!("   - The file has syntax errors");
        
        // Test parsing directly
        println!("\n🧪 Testing direct parsing...");
        let mut parser = gren_lsp_core::Parser::new()?;
        match parser.parse(&workspace.get_document_readonly(&doc_uri).unwrap().text()) {
            Ok(Some(tree)) => {
                println!("✅ Parsing successful");
                let errors = gren_lsp_core::Parser::extract_errors(&tree);
                if errors.is_empty() {
                    println!("✅ No parse errors");
                } else {
                    println!("⚠️  {} parse errors:", errors.len());
                    for error in errors.iter().take(3) {
                        println!("   - {} at {}:{}", error.kind, error.start_position.row, error.start_position.column);
                    }
                }
            }
            Ok(None) => println!("❌ Parser returned no tree"),
            Err(e) => println!("❌ Parse error: {}", e),
        }
    } else {
        println!("✅ Found {} symbols:", symbols.len());
        
        for symbol in &symbols {
            println!("  📍 {} ({:?}) at line {}", 
                symbol.name, symbol.kind, symbol.location.range.start.line + 1);
            if let Some(sig) = &symbol.type_signature {
                println!("      Type: {}", sig);
            }
        }
        
        // Test workspace search
        println!("\n🔍 Testing workspace symbol search...");
        let search_results = workspace.find_symbols("")?;
        println!("✅ Workspace search found {} total symbols", search_results.len());
    }
    
    println!("\n✅ Test complete!");
    Ok(())
}