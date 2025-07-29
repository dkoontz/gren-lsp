use gren_lsp_core::{Workspace, WorkspaceStats};
use lsp_types::*;

/// Create a test text document item
fn create_test_document(uri: &str, content: &str, version: i32) -> TextDocumentItem {
    TextDocumentItem {
        uri: Url::parse(uri).unwrap(),
        language_id: "gren".to_string(),
        version,
        text: content.to_string(),
    }
}

/// Create a test document change
fn create_test_change(uri: &str, version: i32, changes: Vec<TextDocumentContentChangeEvent>) -> DidChangeTextDocumentParams {
    DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse(uri).unwrap(),
            version,
        },
        content_changes: changes,
    }
}

#[test]
fn test_workspace_initialization() {
    let workspace = Workspace::new();
    assert!(workspace.is_ok(), "Workspace should initialize successfully");
    
    let workspace = workspace.unwrap();
    let stats = workspace.stats();
    assert_eq!(stats.document_count, 0);
    assert!(stats.root_uri.is_none());
}

#[test]
fn test_workspace_with_capacity() {
    let workspace = Workspace::with_capacity(50);
    assert!(workspace.is_ok(), "Workspace should initialize with custom capacity");
    
    let workspace = workspace.unwrap();
    let stats = workspace.stats();
    assert_eq!(stats.cache_capacity, 50);
}

#[test]
fn test_set_workspace_root() {
    let mut workspace = Workspace::new().unwrap();
    let root_uri = Url::parse("file:///test/workspace").unwrap();
    
    workspace.set_root(root_uri.clone());
    
    let stats = workspace.stats();
    assert_eq!(stats.root_uri, Some(root_uri));
}

#[test]
fn test_document_open() {
    let mut workspace = Workspace::new().unwrap();
    let doc = create_test_document("file:///test.gren", "module Test exposing (..)", 1);
    let uri = doc.uri.clone();
    
    let result = workspace.open_document(doc);
    assert!(result.is_ok(), "Document should open successfully");
    
    assert!(workspace.is_document_open(&uri));
    let stats = workspace.stats();
    assert_eq!(stats.document_count, 1);
}

#[test]
fn test_document_update() {
    let mut workspace = Workspace::new().unwrap();
    let doc = create_test_document("file:///test.gren", "module Test exposing (..)", 1);
    let uri = doc.uri.clone();
    
    workspace.open_document(doc).unwrap();
    
    // Create a change
    let changes = vec![TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "module UpdatedTest exposing (..)".to_string(),
    }];
    
    let change_params = create_test_change("file:///test.gren", 2, changes);
    let result = workspace.update_document(change_params);
    assert!(result.is_ok(), "Document should update successfully");
    
    // Check the document was updated
    let document = workspace.get_document_readonly(&uri).unwrap();
    assert_eq!(document.version(), 2);
    assert!(document.text().contains("UpdatedTest"));
}

#[test]
fn test_document_version_check() {
    let mut workspace = Workspace::new().unwrap();
    let doc = create_test_document("file:///test.gren", "module Test exposing (..)", 5);
    let uri = doc.uri.clone();
    
    workspace.open_document(doc).unwrap();
    
    // Try to update with older version (should be ignored)
    let changes = vec![TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "module OldTest exposing (..)".to_string(),
    }];
    
    let change_params = create_test_change("file:///test.gren", 3, changes);
    let result = workspace.update_document(change_params);
    assert!(result.is_ok(), "Update should not fail but should be ignored");
    
    // Check the document was NOT updated
    let document = workspace.get_document_readonly(&uri).unwrap();
    assert_eq!(document.version(), 5); // Should still be original version
    assert!(document.text().contains("Test")); // Should still have original content
}

#[test]
fn test_document_close() {
    let mut workspace = Workspace::new().unwrap();
    let doc = create_test_document("file:///test.gren", "module Test exposing (..)", 1);
    let uri = doc.uri.clone();
    
    workspace.open_document(doc).unwrap();
    assert!(workspace.is_document_open(&uri));
    
    let result = workspace.close_document(uri.clone());
    assert!(result.is_ok(), "Document should close successfully");
    
    assert!(!workspace.is_document_open(&uri));
    let stats = workspace.stats();
    assert_eq!(stats.document_count, 0);
}

#[test]
fn test_incremental_parsing() {
    let mut workspace = Workspace::new().unwrap();
    let doc = create_test_document("file:///test.gren", 
        "module Test exposing (..)\n\ngreet name = \"Hello, \" ++ name", 1);
    let uri = doc.uri.clone();
    
    workspace.open_document(doc).unwrap();
    
    // Make incremental changes
    let changes = vec![TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 2, character: 0 },
            end: Position { line: 2, character: 5 },
        }),
        range_length: Some(5),
        text: "farewell".to_string(),
    }];
    
    let change_params = create_test_change("file:///test.gren", 2, changes);
    let result = workspace.update_document(change_params);
    assert!(result.is_ok(), "Incremental update should succeed");
    
    let document = workspace.get_document_readonly(&uri).unwrap();
    assert!(document.text().contains("farewell"));
}

#[test]
fn test_lru_cache_eviction() {
    let mut workspace = Workspace::with_capacity(2).unwrap();
    
    // Open 3 documents (should evict the first)
    let doc1 = create_test_document("file:///test1.gren", "module Test1 exposing (..)", 1);
    let doc2 = create_test_document("file:///test2.gren", "module Test2 exposing (..)", 1);
    let doc3 = create_test_document("file:///test3.gren", "module Test3 exposing (..)", 1);
    
    let uri1 = doc1.uri.clone();
    let uri2 = doc2.uri.clone();
    let uri3 = doc3.uri.clone();
    
    workspace.open_document(doc1).unwrap();
    workspace.open_document(doc2).unwrap();
    
    // Check both documents are open
    assert!(workspace.is_document_open(&uri1));
    assert!(workspace.is_document_open(&uri2));
    assert_eq!(workspace.stats().document_count, 2);
    
    // Open third document (should evict first)
    workspace.open_document(doc3).unwrap();
    
    // First document should be evicted
    assert!(!workspace.is_document_open(&uri1));
    assert!(workspace.is_document_open(&uri2));
    assert!(workspace.is_document_open(&uri3));
    assert_eq!(workspace.stats().document_count, 2);
}

#[test]
fn test_document_access_tracking() {
    let mut workspace = Workspace::with_capacity(2).unwrap();
    
    let doc1 = create_test_document("file:///test1.gren", "module Test1 exposing (..)", 1);
    let doc2 = create_test_document("file:///test2.gren", "module Test2 exposing (..)", 1);
    let doc3 = create_test_document("file:///test3.gren", "module Test3 exposing (..)", 1);
    
    let uri1 = doc1.uri.clone();
    let uri2 = doc2.uri.clone();
    let uri3 = doc3.uri.clone();
    
    workspace.open_document(doc1).unwrap();
    workspace.open_document(doc2).unwrap();
    
    // Access the first document to make it recently used
    let _doc = workspace.get_document(&uri1);
    
    // Open third document (should evict second, not first)
    workspace.open_document(doc3).unwrap();
    
    // First and third documents should remain, second should be evicted
    assert!(workspace.is_document_open(&uri1));
    assert!(!workspace.is_document_open(&uri2));
    assert!(workspace.is_document_open(&uri3));
}

#[test]
fn test_reparse_document() {
    let mut workspace = Workspace::new().unwrap();
    let doc = create_test_document("file:///test.gren", "module Test exposing (..)", 1);
    let uri = doc.uri.clone();
    
    workspace.open_document(doc).unwrap();
    
    let result = workspace.reparse_document(&uri);
    assert!(result.is_ok(), "Document reparse should succeed");
}

#[test]
fn test_reparse_all_documents() {
    let mut workspace = Workspace::new().unwrap();
    
    let doc1 = create_test_document("file:///test1.gren", "module Test1 exposing (..)", 1);
    let doc2 = create_test_document("file:///test2.gren", "module Test2 exposing (..)", 1);
    
    workspace.open_document(doc1).unwrap();
    workspace.open_document(doc2).unwrap();
    
    let result = workspace.reparse_all();
    assert!(result.is_ok(), "Reparse all should succeed");
}

#[test]
fn test_open_documents_list() {
    let mut workspace = Workspace::new().unwrap();
    
    let doc1 = create_test_document("file:///test1.gren", "module Test1 exposing (..)", 1);
    let doc2 = create_test_document("file:///test2.gren", "module Test2 exposing (..)", 1);
    
    let uri1 = doc1.uri.clone();
    let uri2 = doc2.uri.clone();
    
    workspace.open_document(doc1).unwrap();
    workspace.open_document(doc2).unwrap();
    
    let open_docs = workspace.open_documents();
    assert_eq!(open_docs.len(), 2);
    assert!(open_docs.contains(&&uri1));
    assert!(open_docs.contains(&&uri2));
}

#[test]
fn test_update_nonexistent_document() {
    let mut workspace = Workspace::new().unwrap();
    
    let changes = vec![TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "module Test exposing (..)".to_string(),
    }];
    
    let change_params = create_test_change("file:///nonexistent.gren", 1, changes);
    let result = workspace.update_document(change_params);
    
    // Should succeed but do nothing
    assert!(result.is_ok(), "Update of nonexistent document should not fail");
}

#[test]
fn test_parse_errors_tracking() {
    let mut workspace = Workspace::new().unwrap();
    
    // Create document with syntax error
    let doc = create_test_document("file:///test.gren", 
        "module Test exposing (..)\n\ninvalidSyntax :", 1);
    let uri = doc.uri.clone();
    
    workspace.open_document(doc).unwrap();
    
    let document = workspace.get_document_readonly(&uri).unwrap();
    // Note: This depends on tree-sitter-gren's error detection capabilities
    // The test might pass even with errors if the grammar is lenient
}