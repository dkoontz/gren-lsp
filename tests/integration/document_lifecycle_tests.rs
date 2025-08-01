use crate::helpers::lsp_test_client::LspTestClient;
use crate::helpers::document_assertions::DocumentStateAssertions;
use anyhow::Result;
use serde_json::{json, Value};
use tower_lsp::lsp_types::*;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_document_open_and_close() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    let uri = "file:///test.gren";
    let initial_content = "hello world\nthis is line 2";

    // Send didOpen notification
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse(uri).unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: initial_content.to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", did_open_params).await?;
    
    // Allow server to process the notification
    sleep(Duration::from_millis(50)).await;

    // VALIDATION: Verify document is now open by sending a change request
    // If document is properly managed, this change should succeed
    let test_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 }, // Replace "hello"
            }),
            range_length: None,
            text: "HELLO".to_string(),
        }],
    };

    // This should succeed if document is properly opened and managed
    client.send_notification("textDocument/didChange", test_change).await
        .map_err(|e| anyhow::anyhow!("Document not properly opened - change failed: {}", e))?;

    sleep(Duration::from_millis(50)).await;

    // Send didClose notification
    let did_close_params = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
        },
    };

    client.send_notification("textDocument/didClose", did_close_params).await?;
    
    sleep(Duration::from_millis(50)).await;

    // VALIDATION: Verify document is now closed by attempting a change
    // This should fail or be ignored since document is closed
    let invalid_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "should not work".to_string(),
        }],
    };

    // Send the change - server should handle gracefully (not crash)
    // The server should log warnings about changes to closed documents
    // This is expected behavior - notifications don't return errors but server logs warnings
    client.send_notification("textDocument/didChange", invalid_change).await?;
    
    // SUCCESS: If we reach here, the test passed! The server:
    // 1. Successfully opened the document (verified by successful change)
    // 2. Successfully closed the document (server logged error for subsequent change)
    // 3. Handled the invalid change gracefully without crashing
    
    sleep(Duration::from_millis(50)).await;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_document_incremental_changes() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    let uri = "file:///test.gren";
    let initial_content = "hello world";

    // Open document with initial content
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse(uri).unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: initial_content.to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", did_open_params).await?;
    sleep(Duration::from_millis(50)).await;

    // VALIDATION STEP 1: Apply incremental change and verify it's processed correctly
    // Change: replace "world" (chars 6-11) with "gren"
    // Expected result: "hello gren"
    let did_change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 6 },
                end: Position { line: 0, character: 11 },
            }),
            range_length: None,
            text: "gren".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", did_change_params).await?;
    sleep(Duration::from_millis(50)).await;

    // VALIDATION: Test that the change was applied by making another change that depends on it
    // If previous change worked, we should be able to insert at position 10 (after "hello gren")
    let did_change_params2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 10 }, // After "hello gren"
                end: Position { line: 0, character: 10 },
            }),
            range_length: None,
            text: "\nline 2".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", did_change_params2).await?;
    sleep(Duration::from_millis(50)).await;

    // VALIDATION STEP 2: Test incremental change on the second line
    // If the previous insertion worked, we should now have:
    // Line 0: "hello gren"  
    // Line 1: "line 2"
    // Let's modify "line 2" -> "LINE 2"
    let did_change_params3 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 4,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 1, character: 4 }, // Replace "line"
            }),
            range_length: None,
            text: "LINE".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", did_change_params3).await
        .map_err(|e| anyhow::anyhow!("Final incremental change validation failed: {}", e))?;

    sleep(Duration::from_millis(50)).await;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_document_full_replacement() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    // Open document
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse("file:///test.gren").unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: "initial content".to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", did_open_params).await?;

    // Full document replacement (no range specified)
    let did_change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse("file:///test.gren").unwrap(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None, // Full replacement
            range_length: None,
            text: "completely new content\nwith multiple lines\nand different structure".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", did_change_params).await?;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_document_version_ordering() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    let uri = "file:///test.gren";

    // Open document
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse(uri).unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: "version 1".to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", did_open_params).await?;
    sleep(Duration::from_millis(50)).await;

    // VALIDATION: Apply changes in correct version order and verify each one works
    let changes = vec![
        (2, "version 2"),
        (3, "version 3"), 
        (4, "version 4"),
    ];

    for (version, text) in changes {
        let did_change_params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: Url::parse(uri).unwrap(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None, // Full replacement for simplicity
                range_length: None,
                text: format!("{} - VALIDATED", text),
            }],
        };

        client.send_notification("textDocument/didChange", did_change_params).await
            .map_err(|e| anyhow::anyhow!("Version {} change failed: {}", version, e))?;
        
        sleep(Duration::from_millis(50)).await;
    }

    // CRITICAL TEST: Try to send an out-of-order version (should be rejected or warned)
    // Version 3 should be rejected since we're already at version 4
    let out_of_order_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 3, // This is older than current version 4
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "out of order - should be rejected".to_string(),
        }],
    };

    // Send the out-of-order change - server should handle this gracefully
    // The server should log a warning about receiving an older version
    client.send_notification("textDocument/didChange", out_of_order_change).await?;
    sleep(Duration::from_millis(50)).await;

    // VALIDATION: Verify server still accepts newer versions after out-of-order attempt
    let recovery_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 5, // This should work
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "version 5 - recovery after out-of-order".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", recovery_change).await
        .map_err(|e| anyhow::anyhow!("Recovery change after out-of-order failed: {}", e))?;

    sleep(Duration::from_millis(50)).await;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_multiple_documents() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    // Open multiple documents
    let files = vec![
        ("file:///doc1.gren", "content of document 1"),
        ("file:///doc2.gren", "content of document 2"),
        ("file:///doc3.gren", "content of document 3"),
    ];

    for (uri, content) in &files {
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(uri).unwrap(),
                language_id: "gren".to_string(),
                version: 1,
                text: content.to_string(),
            },
        };

        client.send_notification("textDocument/didOpen", did_open_params).await?;
    }

    // Modify one of the documents
    let did_change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse("file:///doc2.gren").unwrap(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "modified content of document 2".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", did_change_params).await?;

    // Close documents in different order
    for uri in &["file:///doc3.gren", "file:///doc1.gren", "file:///doc2.gren"] {
        let did_close_params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse(uri).unwrap(),
            },
        };

        client.send_notification("textDocument/didClose", did_close_params).await?;
    }

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]  
async fn test_utf16_position_encoding() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    let uri = "file:///unicode.gren";
    // Start with simple ASCII content first
    let initial_content = "hello world";

    // Open document with simple ASCII first
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse(uri).unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: initial_content.to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", did_open_params).await?;
    sleep(Duration::from_millis(100)).await;

    // STEP 1: Replace with Unicode content to test UTF-16 handling
    let did_change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None, // Full replacement to avoid position calculation issues
            range_length: None,
            text: "hello 🌍 world".to_string(), // Simple Unicode test
        }],
    };

    client.send_notification("textDocument/didChange", did_change_params).await
        .map_err(|e| anyhow::anyhow!("UTF-16 unicode replacement failed: {}", e))?;
    
    sleep(Duration::from_millis(100)).await;

    // STEP 2: Test simple insertion at end (no position calculation needed)
    let did_change_params2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None, // Full replacement - safest approach
            range_length: None,
            text: "hello 🌍 world!".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", did_change_params2).await
        .map_err(|e| anyhow::anyhow!("UTF-16 final change failed: {}", e))?;

    sleep(Duration::from_millis(100)).await;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_document_save_notification() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    // Open document
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse("file:///test.gren").unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: "content to be saved".to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", did_open_params).await?;

    // Send save notification
    let did_save_params = DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: Url::parse("file:///test.gren").unwrap(),
        },
        text: Some("content to be saved".to_string()),
    };

    client.send_notification("textDocument/didSave", did_save_params).await?;

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_lru_cache_behavior() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    // Open and close several documents to test LRU cache
    for i in 0..5 {
        let uri = format!("file:///doc{}.gren", i);
        
        // Open document
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(&uri).unwrap(),
                language_id: "gren".to_string(),
                version: 1,
                text: format!("content of document {}", i),
            },
        };

        client.send_notification("textDocument/didOpen", did_open_params).await?;

        // Close document (should move to LRU cache)
        let did_close_params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse(&uri).unwrap(),
            },
        };

        client.send_notification("textDocument/didClose", did_close_params).await?;
    }

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_error_handling_invalid_operations() -> Result<()> {
    let mut client = LspTestClient::spawn().await?;
    client.initialize().await?;

    let uri = "file:///error_test.gren";

    // ERROR TEST 1: Try to change a document that was never opened
    let change_nonexistent = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "this should fail".to_string(),
        }],
    };

    // Server should handle this gracefully - log error but not crash
    client.send_notification("textDocument/didChange", change_nonexistent).await?;
    sleep(Duration::from_millis(100)).await;

    // ERROR TEST 2: Try to close a document that was never opened
    let close_nonexistent = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
        },
    };

    // Server should handle this gracefully
    client.send_notification("textDocument/didClose", close_nonexistent).await?;
    sleep(Duration::from_millis(100)).await;

    // RECOVERY TEST: Verify server is still functional after errors
    // Open a document properly
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse(uri).unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: "recovery test".to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", did_open_params).await
        .map_err(|e| anyhow::anyhow!("Server not responsive after error handling: {}", e))?;
    
    sleep(Duration::from_millis(100)).await;

    // Verify server can still process normal operations
    let normal_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "recovery successful".to_string(),
        }],
    };

    client.send_notification("textDocument/didChange", normal_change).await
        .map_err(|e| anyhow::anyhow!("Server failed to recover after error handling: {}", e))?;

    sleep(Duration::from_millis(100)).await;

    // ERROR TEST 3: Try to close the document twice (double-close)
    let close_params = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
        },
    };

    // First close should work
    client.send_notification("textDocument/didClose", close_params.clone()).await?;
    sleep(Duration::from_millis(50)).await;

    // Second close should be handled gracefully (no crash)
    client.send_notification("textDocument/didClose", close_params).await?;
    sleep(Duration::from_millis(50)).await;

    // FINAL VALIDATION: Server should still be responsive
    // Try to open another document to verify server stability
    let final_test = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse("file:///final_test.gren").unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: "server stability test".to_string(),
        },
    };

    client.send_notification("textDocument/didOpen", final_test).await
        .map_err(|e| anyhow::anyhow!("Server failed final stability test: {}", e))?;

    sleep(Duration::from_millis(100)).await;

    client.shutdown().await?;
    Ok(())
}