use crate::{Document, Parser, parse_errors_to_diagnostics};
use anyhow::Result;
use lru::LruCache;
use lsp_types::*;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use tracing::{info, warn};

const DEFAULT_CACHE_SIZE: usize = 100;

pub struct Workspace {
    root_uri: Option<Url>,
    documents: HashMap<Url, Document>,
    recently_accessed: LruCache<Url, ()>,
    parser: Parser,
}

impl Workspace {
    pub fn new() -> Result<Self> {
        Ok(Self {
            root_uri: None,
            documents: HashMap::new(),
            recently_accessed: LruCache::new(
                NonZeroUsize::new(DEFAULT_CACHE_SIZE).unwrap()
            ),
            parser: Parser::new()?,
        })
    }
    
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        Ok(Self {
            root_uri: None,
            documents: HashMap::new(),
            recently_accessed: LruCache::new(
                NonZeroUsize::new(capacity.max(1)).unwrap()
            ),
            parser: Parser::new()?,
        })
    }

    pub fn set_root(&mut self, root_uri: Url) {
        info!("Setting workspace root: {}", root_uri);
        self.root_uri = Some(root_uri);
    }

    pub fn open_document(&mut self, text_document: TextDocumentItem) -> Result<()> {
        let uri = text_document.uri.clone();
        info!("Opening document: {}", uri);
        
        let mut document = Document::new(text_document);
        
        // Trigger initial parse
        document.reparse(&mut self.parser)?;
        
        // Evict old documents if cache is at capacity
        self.evict_if_needed();
        
        // Insert document and update LRU cache
        self.documents.insert(uri.clone(), document);
        self.recently_accessed.put(uri, ());
        
        Ok(())
    }

    pub fn update_document(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.clone();
        
        if let Some(document) = self.documents.get_mut(&uri) {
            // Verify version matches or is newer
            if params.text_document.version < document.version() {
                warn!("Received older version for document {}: {} < {}", 
                      uri, params.text_document.version, document.version());
                return Ok(());
            }
            
            // Apply changes
            document.apply_changes(params.content_changes)?;
            
            // Update access time
            self.recently_accessed.put(uri.clone(), ());
            
            info!("Updated document: {} (version {})", uri, document.version());
        } else {
            warn!("Attempted to update non-existent document: {}", uri);
        }
        
        Ok(())
    }

    pub fn close_document(&mut self, uri: Url) -> Result<()> {
        info!("Closing document: {}", uri);
        
        self.documents.remove(&uri);
        self.recently_accessed.pop(&uri);
        
        Ok(())
    }

    pub fn get_document(&mut self, uri: &Url) -> Option<&mut Document> {
        if self.documents.contains_key(uri) {
            // Update access time
            self.recently_accessed.put(uri.clone(), ());
            self.documents.get_mut(uri)
        } else {
            None
        }
    }

    pub fn get_document_readonly(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Evict least recently used documents if we're at capacity
    fn evict_if_needed(&mut self) {
        while self.documents.len() >= self.recently_accessed.cap().get() {
            if let Some((uri_to_evict, _)) = self.recently_accessed.pop_lru() {
                info!("Evicting document from cache: {}", uri_to_evict);
                self.documents.remove(&uri_to_evict);
            } else {
                break;
            }
        }
    }

    /// Get workspace statistics
    pub fn stats(&self) -> WorkspaceStats {
        WorkspaceStats {
            document_count: self.documents.len(),
            cache_capacity: self.recently_accessed.cap().get(),
            root_uri: self.root_uri.clone(),
        }
    }

    /// Get all open document URIs
    pub fn open_documents(&self) -> Vec<&Url> {
        self.documents.keys().collect()
    }

    /// Check if document is open
    pub fn is_document_open(&self, uri: &Url) -> bool {
        self.documents.contains_key(uri)
    }

    /// Force reparse of a document
    pub fn reparse_document(&mut self, uri: &Url) -> Result<()> {
        if let Some(document) = self.documents.get_mut(uri) {
            document.reparse(&mut self.parser)?;
            self.recently_accessed.put(uri.clone(), ());
        }
        Ok(())
    }

    /// Force reparse of all documents
    pub fn reparse_all(&mut self) -> Result<()> {
        for (uri, document) in &mut self.documents {
            if let Err(e) = document.reparse(&mut self.parser) {
                warn!("Failed to reparse document {}: {}", uri, e);
            }
        }
        Ok(())
    }

    /// Get diagnostics for a document
    pub fn get_diagnostics(&mut self, uri: &Url) -> Vec<Diagnostic> {
        if let Some(document) = self.get_document_readonly(uri) {
            let errors = document.parse_errors().to_vec();
            info!("Document {} has {} parse errors", uri, errors.len());
            let diagnostics = parse_errors_to_diagnostics(errors);
            for diagnostic in &diagnostics {
                info!("Diagnostic: {} at {:?}", diagnostic.message, diagnostic.range);
            }
            diagnostics
        } else {
            warn!("Document not found for diagnostics: {}", uri);
            Vec::new()
        }
    }

    /// Get diagnostics for all open documents
    pub fn get_all_diagnostics(&self) -> HashMap<Url, Vec<Diagnostic>> {
        let mut all_diagnostics = HashMap::new();
        
        for (uri, document) in &self.documents {
            let errors = document.parse_errors().to_vec();
            let diagnostics = parse_errors_to_diagnostics(errors);
            if !diagnostics.is_empty() {
                all_diagnostics.insert(uri.clone(), diagnostics);
            }
        }
        
        all_diagnostics
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceStats {
    pub document_count: usize,
    pub cache_capacity: usize,
    pub root_uri: Option<Url>,
}