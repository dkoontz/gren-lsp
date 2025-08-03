use anyhow::{anyhow, Result};
use lru::LruCache;
use ropey::Rope;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    Position, Range, TextDocumentContentChangeEvent, Url,
};
use tracing::{debug, info, warn};

/// Represents a document managed by the LSP server
#[derive(Debug, Clone)]
pub struct Document {
    /// The URI of the document
    pub uri: Url,
    /// Current version number (from the client)
    pub version: i32,
    /// The document content as a Rope for efficient text operations
    pub content: Rope,
    /// Language identifier (should be "gren" for our files)
    pub language_id: String,
}

impl Document {
    /// Create a new document from didOpen parameters
    pub fn new(uri: Url, version: i32, language_id: String, text: String) -> Self {
        Self {
            uri,
            version,
            content: Rope::from_str(&text),
            language_id,
        }
    }

    /// Apply incremental changes to the document
    pub fn apply_changes(&mut self, version: i32, changes: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        self.version = version;
        
        for change in changes {
            match change.range {
                Some(range) => {
                    // Incremental change
                    self.apply_incremental_change(range, &change.text)?;
                }
                None => {
                    // Full document replacement
                    self.content = Rope::from_str(&change.text);
                }
            }
        }
        
        Ok(())
    }

    /// Apply an incremental change to a specific range
    fn apply_incremental_change(&mut self, range: Range, new_text: &str) -> Result<()> {
        let start_idx = self.position_to_byte_offset(range.start)?;
        let end_idx = self.position_to_byte_offset(range.end)?;
        
        // Validate range
        if start_idx > end_idx {
            return Err(anyhow!("Invalid range: start {} > end {}", start_idx, end_idx));
        }
        
        // Remove the old text
        self.content.remove(start_idx..end_idx);
        
        // Insert the new text
        self.content.insert(start_idx, new_text);
        
        Ok(())
    }

    /// Convert LSP Position to byte offset in the document
    /// LSP uses UTF-16 code units for character positions
    pub fn position_to_byte_offset(&self, position: Position) -> Result<usize> {
        let line = position.line as usize;
        let character = position.character as usize;
        
        // Check if line exists
        if line >= self.content.len_lines() {
            return Err(anyhow!("Line {} out of bounds (document has {} lines)", line, self.content.len_lines()));
        }
        
        let line_start = self.content.line_to_byte(line);
        let line_text = self.content.line(line);
        
        // Convert UTF-16 character offset to byte offset within the line
        let mut utf16_offset = 0;
        let mut byte_offset = 0;
        
        for ch in line_text.chars() {
            if utf16_offset >= character {
                break;
            }
            utf16_offset += ch.len_utf16();
            byte_offset += ch.len_utf8();
        }
        
        Ok(line_start + byte_offset)
    }

    /// Convert byte offset to LSP Position (UTF-16 based)
    pub fn byte_offset_to_position(&self, offset: usize) -> Result<Position> {
        if offset > self.content.len_bytes() {
            return Err(anyhow!("Offset {} out of bounds (document has {} bytes)", offset, self.content.len_bytes()));
        }
        
        let line = self.content.byte_to_line(offset);
        let line_start = self.content.line_to_byte(line);
        let byte_offset_in_line = offset - line_start;
        
        // Convert byte offset to UTF-16 character offset
        let line_text = self.content.line(line);
        let mut utf16_offset = 0;
        let mut current_byte_offset = 0;
        
        for ch in line_text.chars() {
            if current_byte_offset >= byte_offset_in_line {
                break;
            }
            utf16_offset += ch.len_utf16();
            current_byte_offset += ch.len_utf8();
        }
        
        Ok(Position {
            line: line as u32,
            character: utf16_offset as u32,
        })
    }

    /// Get the full text content of the document
    pub fn get_text(&self) -> String {
        self.content.to_string()
    }

    /// Get text within a specific range
    pub fn get_range_text(&self, range: Range) -> Result<String> {
        let start_offset = self.position_to_byte_offset(range.start)?;
        let end_offset = self.position_to_byte_offset(range.end)?;
        
        Ok(self.content.slice(start_offset..end_offset).to_string())
    }
}

/// Manages the lifecycle of all documents opened in the editor
pub struct DocumentManager {
    /// Currently open documents (actively being edited)
    open_documents: HashMap<Url, Document>,
    /// LRU cache for recently closed documents (for performance)
    closed_documents: LruCache<Url, Document>,
}

impl DocumentManager {
    /// Create a new document manager with specified cache size
    pub fn new(cache_size: usize) -> Self {
        Self {
            open_documents: HashMap::new(),
            closed_documents: LruCache::new(NonZeroUsize::new(cache_size).unwrap()),
        }
    }

    /// Handle didOpen notification - add document to active set
    pub fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.clone();
        let document = Document::new(
            params.text_document.uri,
            params.text_document.version,
            params.text_document.language_id,
            params.text_document.text,
        );
        
        info!("Opening document: {} (version {})", uri, document.version);
        
        // Remove from closed cache if it exists
        self.closed_documents.pop(&uri);
        
        // Add to open documents
        self.open_documents.insert(uri, document);
        
        Ok(())
    }

    /// Handle didChange notification - apply incremental changes
    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = &params.text_document.uri;
        let version = params.text_document.version;
        
        debug!("Applying changes to document: {} (version {})", uri, version);
        
        match self.open_documents.get_mut(uri) {
            Some(document) => {
                // Verify version ordering to prevent race conditions
                if version <= document.version {
                    return Err(anyhow!(
                        "Invalid document version ordering. Expected version > {}, received {}",
                        document.version, version
                    ));
                }
                
                document.apply_changes(version, params.content_changes)?;
                debug!("Successfully applied changes to document: {}", uri);
                Ok(())
            }
            None => {
                Err(anyhow!("Attempted to change document that is not open: {}", uri))
            }
        }
    }

    /// Handle didClose notification - move document from active to cache
    pub fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;
        
        info!("Closing document: {}", uri);
        
        match self.open_documents.remove(&uri) {
            Some(document) => {
                // Move to LRU cache for potential future access
                self.closed_documents.put(uri, document);
                Ok(())
            }
            None => {
                warn!("Attempted to close document that is not open: {}", uri);
                Ok(()) // Don't error on double-close
            }
        }
    }

    /// Get an open document by URI
    pub fn get_document(&self, uri: &Url) -> Option<&Document> {
        self.open_documents.get(uri)
    }

    /// Get a mutable reference to an open document
    pub fn get_document_mut(&mut self, uri: &Url) -> Option<&mut Document> {
        self.open_documents.get_mut(uri)
    }

    /// Get document from either open or closed cache
    pub fn get_any_document(&mut self, uri: &Url) -> Option<&Document> {
        if let Some(doc) = self.open_documents.get(uri) {
            Some(doc)
        } else {
            // Check closed cache (this will bump it to front if found)
            self.closed_documents.get(uri)
        }
    }

    /// Get all currently open document URIs
    pub fn get_open_documents(&self) -> Vec<&Url> {
        self.open_documents.keys().collect()
    }

    /// Get cache statistics for monitoring
    pub fn get_stats(&self) -> DocumentManagerStats {
        DocumentManagerStats {
            open_documents: self.open_documents.len(),
            cached_documents: self.closed_documents.len(),
        }
    }

    /// Test-only method: Check if a document is currently open
    pub fn is_document_open(&self, uri: &Url) -> bool {
        self.open_documents.contains_key(uri)
    }

    /// Test-only method: Check if a document is in the closed cache
    pub fn is_document_cached(&mut self, uri: &Url) -> bool {
        self.closed_documents.contains(uri)
    }

    /// Get document content by URI (from open documents only for LSP compilation)
    pub fn get_open_document_content(&self, uri: &Url) -> Option<String> {
        self.open_documents.get(uri).map(|doc| doc.get_text())
    }

    /// Test-only method: Get document content by URI (from open or cached)
    pub fn get_document_content(&mut self, uri: &Url) -> Option<String> {
        if let Some(doc) = self.open_documents.get(uri) {
            Some(doc.get_text())
        } else {
            self.closed_documents.get(uri).map(|doc| doc.get_text())
        }
    }

    /// Test-only method: Get document version by URI
    pub fn get_document_version(&mut self, uri: &Url) -> Option<i32> {
        if let Some(doc) = self.open_documents.get(uri) {
            Some(doc.version)
        } else {
            self.closed_documents.get(uri).map(|doc| doc.version)
        }
    }

    /// Test-only method: Get the current cache capacity and usage
    pub fn get_cache_info(&self) -> (usize, usize) {
        (self.closed_documents.cap().get(), self.closed_documents.len())
    }

    /// Get all open documents with their content for workspace operations
    pub fn get_all_open_documents(&self) -> HashMap<Url, String> {
        self.open_documents
            .iter()
            .map(|(uri, document)| (uri.clone(), document.get_text()))
            .collect()
    }
}

/// Statistics about document manager state
#[derive(Debug, Clone)]
pub struct DocumentManagerStats {
    pub open_documents: usize,
    pub cached_documents: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new(
            Url::parse("file:///test.gren").unwrap(),
            1,
            "gren".to_string(),
            "hello world".to_string(),
        );
        
        assert_eq!(doc.version, 1);
        assert_eq!(doc.get_text(), "hello world");
        assert_eq!(doc.language_id, "gren");
    }

    #[test]
    fn test_position_conversion() {
        let doc = Document::new(
            Url::parse("file:///test.gren").unwrap(),
            1,
            "gren".to_string(),
            "hello\nworld\n".to_string(),
        );
        
        // Test position at start of document
        let pos = Position { line: 0, character: 0 };
        assert_eq!(doc.position_to_byte_offset(pos).unwrap(), 0);
        
        // Test position at start of second line
        let pos = Position { line: 1, character: 0 };
        assert_eq!(doc.position_to_byte_offset(pos).unwrap(), 6); // "hello\n" = 6 bytes
        
        // Test byte offset to position conversion
        let pos = doc.byte_offset_to_position(6).unwrap();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_incremental_changes() {
        let mut doc = Document::new(
            Url::parse("file:///test.gren").unwrap(),
            1,
            "gren".to_string(),
            "hello world".to_string(),
        );
        
        // Replace "world" with "gren"
        let range = Range {
            start: Position { line: 0, character: 6 },
            end: Position { line: 0, character: 11 },
        };
        
        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(range),
            range_length: None,
            text: "gren".to_string(),
        }];
        
        doc.apply_changes(2, changes).unwrap();
        assert_eq!(doc.get_text(), "hello gren");
        assert_eq!(doc.version, 2);
    }
}