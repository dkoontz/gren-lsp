// Debug the type deduplication logic
use gren_lsp_core::{Symbol as GrenSymbol, Workspace};
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Debugging type deduplication logic");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));

    let test_content = r#"module Bytes exposing (..)

type Bytes = Bytes

empty : Bytes
empty = Bytes"#;

    let doc_uri = Url::parse("file:///test/Bytes.gren")?;
    let doc_item = TextDocumentItem {
        uri: doc_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: test_content.to_string(),
    };

    {
        let mut workspace = workspace.write().await;
        workspace.open_document(doc_item)?;
    }

    // Get raw symbols to see what we're starting with
    let workspace = workspace.read().await;
    let symbols = workspace.get_file_symbols(&doc_uri)?;

    println!("📋 Raw symbols ({}): ", symbols.len());
    for (i, symbol) in symbols.iter().enumerate() {
        println!("  {}: '{}' ({:?})", i + 1, symbol.name, symbol.kind,);
    }

    // Manually simulate the deduplication logic
    println!("\n🔧 Simulating deduplication logic:");

    let mut processed_types = std::collections::HashSet::new();
    let mut types = Vec::new();

    // Group symbols by type
    for symbol in &symbols {
        if symbol.kind == SymbolKind::CLASS {
            types.push(symbol);
        }
    }

    println!("  Found {} type symbols", types.len());
    for (i, typ) in types.iter().enumerate() {
        println!("    {}: '{}'", i + 1, typ.name);
    }

    // Sort types to process simple names first
    types.sort_by(|a, b| {
        let a_is_verbose = a.name.starts_with("type ");
        let b_is_verbose = b.name.starts_with("type ");
        a_is_verbose.cmp(&b_is_verbose)
    });

    println!("  After sorting:");
    for (i, typ) in types.iter().enumerate() {
        println!("    {}: '{}'", i + 1, typ.name);
    }

    // Process each type
    for (i, typ) in types.iter().enumerate() {
        let type_name = if typ.name.starts_with("type ") {
            if let Some(name_part) = typ.name.split_whitespace().nth(1) {
                name_part
                    .split('=')
                    .next()
                    .unwrap_or(name_part)
                    .trim()
                    .to_string()
            } else {
                continue;
            }
        } else {
            typ.name.clone()
        };

        println!(
            "  Processing type #{}: '{}' -> extracted name: '{}'",
            i + 1,
            typ.name,
            type_name
        );

        if processed_types.contains(&type_name) {
            println!("    ❌ SKIPPED: '{}' already processed", type_name);
        } else {
            println!("    ✅ ADDED: '{}'", type_name);
            processed_types.insert(type_name);
        }
    }

    println!("\n📊 Final processed types: {}", processed_types.len());
    for name in &processed_types {
        println!("  - '{}'", name);
    }

    Ok(())
}
