// Test LSP symbol protocol methods
use gren_lsp_core::Workspace;
use gren_lsp_protocol::handlers::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();
    
    println!("🧪 Testing LSP Symbol Protocol Methods");
    
    // Create workspace and add a test document
    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());
    
    // Test Gren source with various symbols
    let test_source = r#"
module SymbolTest exposing (..)

{-| User configuration -}
type alias Config = 
    { apiUrl : String
    , timeout : Int
    , retries : Int
    }

{-| Request status -}
type RequestStatus 
    = Pending
    | Loading String
    | Success String 
    | Failed String

{-| Initialize configuration -}
initConfig : String -> Config
initConfig url = 
    { apiUrl = url
    , timeout = 5000
    , retries = 3
    }

{-| Process a request -}
processRequest : Config -> String -> RequestStatus
processRequest config endpoint =
    case endpoint of
        "" -> Failed "Empty endpoint"
        _ -> Loading ("Fetching " ++ endpoint)

{-| Handle response -}
handleResponse : RequestStatus -> String
handleResponse status =
    case status of
        Pending -> "Waiting..."
        Loading msg -> "Loading: " ++ msg
        Success data -> "Success: " ++ data
        Failed error -> "Error: " ++ error
"#;

    // Add document to workspace
    let doc_uri = Url::parse("file:///test_symbols.gren")?;
    let doc_item = TextDocumentItem {
        uri: doc_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: test_source.to_string(),
    };
    
    {
        let mut ws = workspace.write().await;
        ws.open_document(doc_item)?;
    }
    
    println!("📄 Document added to workspace");
    
    // Test 1: Document Symbols
    println!("\n🔍 Testing textDocument/documentSymbol...");
    let doc_symbol_params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: doc_uri.clone(),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    
    match handlers.document_symbols(doc_symbol_params).await {
        Ok(Some(DocumentSymbolResponse::Nested(symbols))) => {
            println!("✅ Found {} document symbols:", symbols.len());
            for symbol in &symbols {
                println!("  📋 {} ({:?}) at line {}", 
                    symbol.name, symbol.kind, symbol.range.start.line);
                if let Some(detail) = &symbol.detail {
                    println!("      Type: {}", detail);
                }
                if let Some(children) = &symbol.children {
                    for child in children {
                        println!("    ├─ {} ({:?})", child.name, child.kind);
                    }
                }
            }
        }
        Ok(Some(DocumentSymbolResponse::Flat(symbols))) => {
            println!("✅ Found {} flat document symbols:", symbols.len());
            for symbol in &symbols {
                println!("  📋 {} ({:?})", symbol.name, symbol.kind);
            }
        }
        Ok(None) => {
            println!("❌ No document symbols found");
        }
        Err(e) => {
            println!("❌ Document symbols error: {:?}", e);
        }
    }
    
    // Test 2: Workspace Symbol Search
    println!("\n🔍 Testing workspace/symbol...");
    
    let test_queries = vec!["Config", "process", "Request", "init", ""];
    
    for query in test_queries {
        println!("\n  Query: '{}'", query);
        let workspace_symbol_params = WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        
        match handlers.workspace_symbols(workspace_symbol_params).await {
            Ok(Some(symbols)) => {
                println!("    ✅ Found {} symbols:", symbols.len());
                for symbol in symbols.iter().take(5) { // Show first 5
                    println!("      🔍 {} ({:?}) in {}", 
                        symbol.name, symbol.kind, symbol.location.uri);
                }
                if symbols.len() > 5 {
                    println!("      ... and {} more", symbols.len() - 5);
                }
            }
            Ok(None) => {
                println!("    ❌ No symbols found");
            }
            Err(e) => {
                println!("    ❌ Workspace symbols error: {:?}", e);
            }
        }
    }
    
    println!("\n✅ LSP Symbol Protocol Methods Test Complete!");
    Ok(())
}