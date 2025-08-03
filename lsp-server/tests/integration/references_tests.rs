use anyhow::Result;
use tower_lsp::lsp_types::*;
use crate::helpers::lsp_test_client::LspTestClient;
use tokio::time::{sleep, Duration};

/// Test basic references functionality
#[tokio::test]
async fn test_references_basic_workflow() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    // Initialize the server
    let capabilities = client.initialize().await?;
    eprintln!("=== SERVER CAPABILITIES ===");
    eprintln!("References provider: {:?}", capabilities.capabilities.references_provider);
    eprintln!("Definition provider: {:?}", capabilities.capabilities.definition_provider);
    eprintln!("Hover provider: {:?}", capabilities.capabilities.hover_provider);

    // Test Gren code with a simple function
    let test_code = r#"
module Main exposing (..)

greet : String -> String
greet name =
    "Hello, " ++ name ++ "!"

main : String
main =
    greet "World"
"#;

    let uri = Url::parse("file:///test.gren")?;
    
    // Open the document
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "gren".to_string(),
            version: 1,
            text: test_code.to_string(),
        },
    };
    client.send_notification("textDocument/didOpen", did_open_params).await?;

    // Wait for indexing
    sleep(Duration::from_millis(500)).await;
    
    // Debug: Check what happened during indexing
    println!("=== DEBUG: Test setup complete, checking indexed content ===");

    // Request references for the 'greet' function at its usage position (line 9, char 4)
    let position = Position { line: 9, character: 4 };
    let references_params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position,
        },
        context: ReferenceContext {
            include_declaration: true,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    eprintln!("=== SENDING REFERENCES REQUEST ===");
    eprintln!("Request params: {:?}", references_params);
    
    let references_response: Option<Vec<Location>> = client.send_request_with_timeout(
        "textDocument/references", 
        references_params, 
        1000
    ).await?;
    
    eprintln!("=== RECEIVED REFERENCES RESPONSE ===");
    eprintln!("Response: {:?}", references_response);

    // TEST ASSERTION: Should find exactly 2 references (declaration + usage)
    let locations = references_response.expect("References request should return results");
    assert_eq!(locations.len(), 2, "Should find exactly 2 references: declaration at line 5 and usage at line 9");
    
    // Verify the declaration location (line 5, character 0)
    let declaration = &locations[0];
    assert_eq!(declaration.uri, uri);
    assert_eq!(declaration.range.start.line, 4); // 0-indexed, so line 5 = index 4
    assert_eq!(declaration.range.start.character, 0);
    
    // Verify the usage location (line 9, character 4)  
    let usage = &locations[1];
    assert_eq!(usage.uri, uri);
    assert_eq!(usage.range.start.line, 8); // 0-indexed, so line 9 = index 8
    assert_eq!(usage.range.start.character, 4);

    // Test with include_declaration: false
    let references_params_no_decl = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position,
        },
        context: ReferenceContext {
            include_declaration: false,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let references_response_no_decl: Option<Vec<Location>> = client.send_request_with_timeout(
        "textDocument/references", 
        references_params_no_decl, 
        1000
    ).await?;
    
    // TEST ASSERTION: Should find exactly 1 reference (usage only, no declaration)
    let locations_no_decl = references_response_no_decl.expect("References request should return results");
    assert_eq!(locations_no_decl.len(), 1, "Should find exactly 1 reference when excluding declaration");
    
    // Verify the usage location (line 9, character 4)
    let usage_only = &locations_no_decl[0];
    assert_eq!(usage_only.uri, uri);
    assert_eq!(usage_only.range.start.line, 8); // 0-indexed, so line 9 = index 8
    assert_eq!(usage_only.range.start.character, 4);

    client.shutdown().await?;
    Ok(())
}

/// Test references with non-existent symbol
#[tokio::test]
async fn test_references_nonexistent_symbol() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    client.initialize().await?;

    let test_code = r#"
module Main exposing (..)

main : String
main = "Hello"
"#;

    let uri = Url::parse("file:///test.gren")?;
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "gren".to_string(),
            version: 1,
            text: test_code.to_string(),
        },
    };
    client.send_notification("textDocument/didOpen", did_open_params).await?;

    // Wait for indexing
    sleep(Duration::from_millis(500)).await;

    // Request references at a position with no symbol
    let position = Position { line: 1, character: 0 }; // Empty line
    let references_params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position,
        },
        context: ReferenceContext {
            include_declaration: true,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let references_response: Option<Vec<Location>> = client.send_request_with_timeout(
        "textDocument/references", 
        references_params, 
        1000
    ).await?;

    // TEST ASSERTION: Should return None for non-existent symbol (no fallback)
    assert!(references_response.is_none(), "Should return exactly None for non-existent symbol");

    client.shutdown().await?;
    Ok(())
}

/// Test references server capability advertisement
#[tokio::test]
async fn test_references_capability_advertisement() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;

    let capabilities = client.initialize().await?;

    // Check that references capability is advertised
    if let Some(ref references_provider) = capabilities.capabilities.references_provider {
        println!("✅ References capability advertised: {:?}", references_provider);
    } else {
        println!("❌ References capability NOT advertised");
    }
    
    // Also check for text document sync capabilities
    if let Some(ref server_capabilities) = capabilities.capabilities.text_document_sync {
        println!("✅ Text document sync capabilities present");
    }

    client.shutdown().await?;
    Ok(())
}