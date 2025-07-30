// Full pipeline test: parsing -> symbol extraction -> database storage
use gren_lsp_core::Workspace;
use lsp_types::*;

fn main() -> anyhow::Result<()> {
    // Create a workspace
    let mut workspace = Workspace::new()?;

    // Test Gren source code
    let gren_source = r#"
module TestPipeline exposing (..)

type alias User = { name : String, id : Int }
type Status = Online | Offline

greet : String -> String
greet name = "Hello, " ++ name

getUserStatus : User -> Status  
getUserStatus user = if user.id > 0 then Online else Offline
"#;

    // Create a text document
    let doc_item = TextDocumentItem {
        uri: Url::parse("file:///test_pipeline.gren")?,
        language_id: "gren".to_string(),
        version: 1,
        text: gren_source.to_string(),
    };

    println!("🔄 Opening document and extracting symbols...");
    workspace.open_document(doc_item)?;

    println!("🔍 Searching for symbols in the workspace...");
    let all_symbols = workspace.find_symbols("")?; // Empty string finds all symbols

    println!("\n📊 Extracted {} total symbols:", all_symbols.len());
    for symbol in &all_symbols {
        println!("  {} ({:?})", symbol.name, symbol.kind);
        if let Some(sig) = &symbol.type_signature {
            println!("    Type: {}", sig);
        }
    }

    println!("\n🔍 Testing specific symbol searches:");

    // Test function search
    let greet_symbols = workspace.find_symbols("greet")?;
    println!("  'greet' search: {} results", greet_symbols.len());

    // Test type search
    let user_symbols = workspace.find_symbols("User")?;
    println!("  'User' search: {} results", user_symbols.len());

    // Test partial search
    let get_symbols = workspace.find_symbols("get")?;
    println!("  'get' search: {} results", get_symbols.len());

    println!("\n✅ Full pipeline test completed successfully!");

    Ok(())
}
