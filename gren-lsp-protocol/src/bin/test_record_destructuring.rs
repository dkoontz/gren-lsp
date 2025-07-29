// Test record destructuring parsing issue
use gren_lsp_core::Workspace;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing record destructuring parsing");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));

    // Test the problematic record destructuring pattern
    let test_content = r#"module Test exposing (..)

arrayHelp : Array a -> Int -> (Seed -> { value : a, seed : Seed }) -> Seed -> { value : Array a, seed : Seed }
arrayHelp revArray n gen seed =
  if n < 1 then
    { value = revArray, seed = seed}
  else
    let
      { value = value, seed = newSeed } =
        gen seed
    in
      arrayHelp (Array.pushFirst value revArray) (n-1) gen newSeed
"#;

    let doc_uri = Url::parse("file:///test/RecordTest.gren")?;
    let doc_item = TextDocumentItem {
        uri: doc_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: test_content.to_string(),
    };

    // Add document to workspace and see if we get parse errors
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(doc_item)?;
    }

    // Get diagnostics to see the parse error
    let mut workspace = workspace.write().await;
    let diagnostics = workspace.get_diagnostics(&doc_uri);
    println!("📋 Found {} diagnostics:", diagnostics.len());
    for (i, diagnostic) in diagnostics.iter().enumerate() {
        println!(
            "  {}: {} (line {}, char {})",
            i + 1,
            diagnostic.message,
            diagnostic.range.start.line,
            diagnostic.range.start.character
        );
    }

    println!("\n🧪 Test complete!");
    Ok(())
}
