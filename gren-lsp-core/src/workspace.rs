use crate::{parse_errors_to_diagnostics, Document, Parser, SymbolExtractor, SymbolIndex};
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
    symbol_index: SymbolIndex,
    symbol_extractor: SymbolExtractor,
}

impl Workspace {
    pub fn new() -> Result<Self> {
        Ok(Self {
            root_uri: None,
            documents: HashMap::new(),
            recently_accessed: LruCache::new(NonZeroUsize::new(DEFAULT_CACHE_SIZE).unwrap()),
            parser: Parser::new()?,
            symbol_index: SymbolIndex::new()?,
            symbol_extractor: SymbolExtractor::new()?,
        })
    }

    pub fn with_capacity(capacity: usize) -> Result<Self> {
        Ok(Self {
            root_uri: None,
            documents: HashMap::new(),
            recently_accessed: LruCache::new(NonZeroUsize::new(capacity.max(1)).unwrap()),
            parser: Parser::new()?,
            symbol_index: SymbolIndex::new()?,
            symbol_extractor: SymbolExtractor::new()?,
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

        // Insert document first, then extract symbols
        self.documents.insert(uri.clone(), document);

        // Extract and index symbols
        self.extract_and_update_symbols_for_uri(&uri)?;

        // Evict old documents if cache is at capacity
        self.evict_if_needed();

        // Update LRU cache
        self.recently_accessed.put(uri, ());

        Ok(())
    }

    pub fn update_document(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.clone();

        if let Some(document) = self.documents.get_mut(&uri) {
            // Verify version matches or is newer
            if params.text_document.version < document.version() {
                warn!(
                    "Received older version for document {}: {} < {}",
                    uri,
                    params.text_document.version,
                    document.version()
                );
                return Ok(());
            }

            // Apply changes
            document.apply_changes(params.content_changes)?;

            // Update access time
            self.recently_accessed.put(uri.clone(), ());

            info!("Updated document: {} (version {})", uri, document.version());
        }

        // Re-extract and index symbols after releasing the borrow
        if self.documents.contains_key(&uri) {
            if let Err(e) = self.extract_and_update_symbols_for_uri(&uri) {
                warn!("Failed to re-extract symbols for {}: {}", uri, e);
            }
        }

        if !self.documents.contains_key(&uri) {
            warn!("Attempted to update non-existent document: {}", uri);
        }

        Ok(())
    }

    pub fn close_document(&mut self, uri: Url) -> Result<()> {
        info!("Closing document: {}", uri);

        // Remove symbols from index
        if let Err(e) = self.symbol_index.clear_file_symbols(uri.as_str()) {
            warn!("Failed to clear symbols for {}: {}", uri, e);
        }

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
        // First check if document exists
        if !self.documents.contains_key(uri) {
            warn!("Document not found for diagnostics: {}", uri);
            return Vec::new();
        }

        // Get document and ensure it's reparsed if needed
        if let Some(document) = self.documents.get_mut(uri) {
            info!("Getting parse tree for diagnostics: {}", uri);
            if let Err(e) = document.get_parse_tree(&mut self.parser) {
                warn!("Failed to parse document {}: {}", uri, e);
                return Vec::new();
            }

            // Update LRU access time
            self.recently_accessed.put(uri.clone(), ());

            // Get diagnostics from the same document reference
            let errors = document.parse_errors().to_vec();
            info!("Document {} has {} parse errors", uri, errors.len());
            let diagnostics = parse_errors_to_diagnostics(errors);
            for diagnostic in &diagnostics {
                info!(
                    "Diagnostic: {} at {:?}",
                    diagnostic.message, diagnostic.range
                );
            }
            diagnostics
        } else {
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

    /// Extract and index symbols from a document
    fn extract_and_update_symbols_for_uri(&mut self, uri: &Url) -> Result<()> {
        // Clear existing symbols for this file first
        if let Err(e) = self.symbol_index.clear_file_symbols(uri.as_str()) {
            warn!("Failed to clear symbols for {}: {}", uri, e);
        }

        // Get document content and parse tree in separate scopes to avoid borrowing conflicts
        let (source, tree_available) = if let Some(document) = self.documents.get_mut(uri) {
            let source = document.text().to_string();
            let tree_result = document.get_parse_tree(&mut self.parser);
            match tree_result {
                Ok(Some(_)) => (source, true),
                Ok(None) => (source, false),
                Err(e) => {
                    warn!("Failed to get parse tree for {}: {}", uri, e);
                    return Ok(());
                }
            }
        } else {
            warn!("Document not found for symbol extraction: {}", uri);
            return Ok(());
        };

        if !tree_available {
            warn!("No parse tree available for symbol extraction: {}", uri);
            return Ok(());
        }

        // Get the tree again in a separate borrow scope
        if let Some(document) = self.documents.get_mut(uri) {
            if let Ok(Some(tree)) = document.get_parse_tree(&mut self.parser) {
                // Extract symbols
                match self.symbol_extractor.extract_symbols(tree, &source, uri) {
                    Ok(symbols) => {
                        info!("Extracted {} symbols from {}", symbols.len(), uri);

                        // Index each symbol
                        for symbol in symbols {
                            if let Err(e) = self.symbol_index.index_symbol(&symbol) {
                                warn!(
                                    "Failed to index symbol '{}' from {}: {}",
                                    symbol.name, uri, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to extract symbols from {}: {}", uri, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Search for symbols by name
    pub fn find_symbols(&self, name: &str) -> Result<Vec<crate::Symbol>> {
        match self.symbol_index.find_symbol(name) {
            Ok(symbols) => Ok(symbols),
            Err(e) => {
                warn!("Failed to search symbols for '{}': {}", name, e);
                Ok(Vec::new())
            }
        }
    }

    /// Get all symbols for a specific file
    pub fn get_file_symbols(&self, uri: &Url) -> Result<Vec<crate::Symbol>> {
        // For now, we'll do a full search and filter
        // TODO: Add a more efficient method to the symbol index
        match self.symbol_index.find_symbol("") {
            Ok(all_symbols) => {
                let file_symbols = all_symbols
                    .into_iter()
                    .filter(|symbol| symbol.location.uri == *uri)
                    .collect();
                Ok(file_symbols)
            }
            Err(e) => {
                warn!("Failed to get symbols for {}: {}", uri, e);
                Ok(Vec::new())
            }
        }
    }

    /// Force re-indexing of all open documents
    pub fn reindex_all_symbols(&mut self) -> Result<()> {
        info!("Re-indexing symbols for all open documents");

        let uris: Vec<Url> = self.documents.keys().cloned().collect();
        for uri in uris {
            // Extract and index symbols for each document separately to avoid borrowing issues
            if self.documents.contains_key(&uri) {
                if let Err(e) = self.extract_and_update_symbols_for_uri(&uri) {
                    warn!("Failed to re-index symbols for {}: {}", uri, e);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceStats {
    pub document_count: usize,
    pub cache_capacity: usize,
    pub root_uri: Option<Url>,
}
