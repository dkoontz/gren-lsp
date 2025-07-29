// Test what tree-sitter gives us for qualified function calls
use gren_lsp_core::Workspace;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing parsing of qualified function calls");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));

    // Test content with qualified function calls
    let test_content = r#"module Test exposing (..)

import Gren.Kernel.Bytes

flatten : Array Bytes -> Bytes
flatten =
    Gren.Kernel.Bytes.flatten

example : String -> String  
example str =
    let
        result = String.length str
        other = Utils.isEmpty str
    in
        Gren.String.fromBytes result
"#;

    let doc_uri = Url::parse("file:///test/QualifiedTest.gren")?;
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

    // Get the document and look at what lines contain qualified calls
    println!("📋 Content analysis:");
    for (line_num, line) in test_content.lines().enumerate() {
        let line = line.trim();
        if line.contains('.') && !line.starts_with("import") && !line.starts_with("--") {
            println!("  Line {}: '{}'", line_num + 1, line);

            // Try to parse the qualified structure manually
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if part.contains('.') && !part.starts_with(".") {
                    // This looks like a qualified identifier
                    let segments: Vec<&str> = part.split('.').collect();
                    if segments.len() > 1 {
                        println!("    Qualified identifier: '{}'", part);
                        println!("      Module path: {:?}", &segments[..segments.len() - 1]);
                        println!("      Function name: '{}'", segments[segments.len() - 1]);
                    }
                }
            }
        }
    }

    // Now let's see what symbols were extracted
    let workspace = workspace.read().await;
    let symbols = workspace.get_file_symbols(&doc_uri)?;

    println!("\n🎯 Extracted symbols:");
    for symbol in &symbols {
        println!(
            "  '{}' ({:?}) at line {}",
            symbol.name, symbol.kind, symbol.location.range.start.line
        );
    }

    // Test what happens when we search for "flatten"
    println!("\n🔍 Searching for 'flatten':");
    let flatten_results = workspace.find_symbols("flatten")?;
    for result in &flatten_results {
        println!(
            "  Found '{}' in {} at line {}",
            result.name, result.location.uri, result.location.range.start.line
        );
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
