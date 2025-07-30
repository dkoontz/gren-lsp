use crate::{
    compiler_diagnostics_to_lsp, parse_errors_to_diagnostics, Document,
    GrenCompiler, Parser, SymbolExtractor, SymbolIndex,
};
use anyhow::Result;
use lru::LruCache;
use lsp_types::*;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

const DEFAULT_CACHE_SIZE: usize = 100;

pub struct Workspace {
    root_uri: Option<Url>,
    documents: HashMap<Url, Document>,
    recently_accessed: LruCache<Url, ()>,
    parser: Parser,
    symbol_index: SymbolIndex,
    symbol_extractor: SymbolExtractor,
    compiler: Option<GrenCompiler>,
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
            compiler: None,
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
            compiler: None,
        })
    }

    pub fn set_root(&mut self, root_uri: Url) {
        info!("Setting workspace root: {}", root_uri);
        self.root_uri = Some(root_uri.clone());
        
        // Try to initialize the compiler when root is set
        if let Ok(path) = uri_to_path(&root_uri) {
            match GrenCompiler::new(path) {
                Ok(compiler) => {
                    if compiler.is_available() {
                        info!("Gren compiler initialized for workspace");
                        self.compiler = Some(compiler);
                    } else {
                        warn!("Gren compiler not available");
                    }
                }
                Err(e) => {
                    warn!("Failed to initialize Gren compiler: {}", e);
                }
            }
        }
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

        // Update LRU cache BEFORE evicting to ensure proper LRU ordering
        self.recently_accessed.put(uri, ());

        // Evict old documents if cache is at capacity
        self.evict_if_needed();

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

        // Do NOT remove symbols from index when closing documents
        // Symbols should persist to support cross-file references
        // Only remove the document from memory cache
        self.documents.remove(&uri);
        self.recently_accessed.pop(&uri);

        Ok(())
    }

    /// Remove a file entirely (e.g., when deleted from filesystem)
    /// This removes both the document and its symbols
    pub fn remove_file(&mut self, uri: Url) -> Result<()> {
        info!("Removing file completely: {}", uri);

        // Remove symbols from index for deleted files
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
        while self.documents.len() > self.recently_accessed.cap().get() {
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
            open_documents: self.documents.keys().cloned().collect(),
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

    /// Get diagnostics for a document (syntax only - for backward compatibility)
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

    /// Export parse trees for all open documents to the specified directory
    pub fn export_all_parse_trees(&self, export_dir: &Path) -> Result<()> {
        info!("Exporting parse trees for {} documents to {}", 
              self.documents.len(), export_dir.display());
        
        // Create the export directory if it doesn't exist
        std::fs::create_dir_all(export_dir)?;
        
        let mut exported_count = 0;
        let mut error_count = 0;
        
        for (uri, document) in &self.documents {
            match document.export_parse_tree(export_dir) {
                Ok(()) => {
                    exported_count += 1;
                }
                Err(e) => {
                    warn!("Failed to export parse tree for {}: {}", uri, e);
                    error_count += 1;
                }
            }
        }
        
        info!("Parse tree export completed: {} exported, {} errors", 
              exported_count, error_count);
        
        Ok(())
    }

    /// Export parse tree for a specific document
    pub fn export_parse_tree_for_document(&self, uri: &Url, export_dir: &Path) -> Result<()> {
        if let Some(document) = self.documents.get(uri) {
            // Create the export directory if it doesn't exist
            std::fs::create_dir_all(export_dir)?;
            
            document.export_parse_tree(export_dir)?;
            info!("Parse tree exported for document: {}", uri);
        } else {
            warn!("Document not found for parse tree export: {}", uri);
        }
        
        Ok(())
    }

    /// Compile a document using the Gren compiler
    /// Prefers in-memory content for real-time diagnostics, falls back to disk file
    pub async fn compile_document(&mut self, uri: &Url) -> Result<crate::compiler::CompilationResult> {
        if let Some(ref mut compiler) = self.compiler {
            if let Ok(path) = uri_to_path(uri) {
                // Prefer in-memory content if document is open in the workspace
                // This provides real-time diagnostics for unsaved changes
                if let Some(document) = self.documents.get(uri) {
                    info!("💭 Compiling in-memory content for real-time diagnostics: {}", path.display());
                    return compiler.compile_content(document.text(), &path).await;
                } else if path.exists() {
                    // Fall back to disk file if not in workspace
                    info!("🔨 Compiling disk file: {}", path.display());
                    return compiler.compile_file(&path).await;
                } else {
                    info!("⚠️  No document or file found for: {}", path.display());
                    // Return empty result - no content available
                    return Ok(crate::compiler::CompilationResult {
                        success: true,
                        diagnostics: Vec::new(),
                        timestamp: std::time::SystemTime::now(),
                        content_hash: 0,
                    });
                }
            }
        }
        
        anyhow::bail!("Compiler not available or invalid URI")
    }

    /// Get compiler diagnostics for all open documents
    pub async fn get_compiler_diagnostics(&mut self) -> HashMap<Url, Vec<crate::compiler::CompilerDiagnostic>> {
        let mut diagnostics = HashMap::new();
        
        if self.compiler.is_none() {
            return diagnostics;
        }

        // Collect URIs to avoid borrowing issues
        let uris: Vec<Url> = self.documents.keys().cloned().collect();
        
        for uri in uris {
            if let Ok(result) = self.compile_document(&uri).await {
                if !result.diagnostics.is_empty() {
                    diagnostics.insert(uri, result.diagnostics);
                }
            }
        }
        
        diagnostics
    }

    /// Check if compiler is available
    pub fn has_compiler(&self) -> bool {
        self.compiler.as_ref().map_or(false, |c| c.is_available())
    }

    /// Invalidate compiler cache when project configuration changes
    pub fn invalidate_compiler_cache(&mut self) {
        if let Some(ref mut compiler) = self.compiler {
            compiler.invalidate_all_cache();
            info!("🔄 Invalidated all compiler cache due to project changes");
        }
    }

    /// Force refresh diagnostics for a specific document
    pub async fn force_refresh_diagnostics(&mut self, uri: &Url) -> Result<Vec<Diagnostic>> {
        info!("🔄 Force refreshing diagnostics for {}", uri);
        
        // Invalidate compiler cache for this file
        if let Some(ref mut compiler) = self.compiler {
            if let Ok(path) = uri_to_path(uri) {
                compiler.invalidate_cache(&path);
            }
        }
        
        // Get fresh diagnostics
        self.get_document_diagnostics(uri).await
    }

    /// Detect if a file is a project configuration file that should trigger cache invalidation
    pub fn is_project_file(&self, uri: &Url) -> bool {
        if let Ok(path) = uri_to_path(uri) {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                matches!(filename, "gren.json" | "gren-package.json")
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get compiler diagnostics for a document
    pub async fn get_document_diagnostics(&mut self, uri: &Url) -> Result<Vec<Diagnostic>> {
        // Use only compiler diagnostics - they provide comprehensive and accurate error messages
        if self.has_compiler() {
            match self.compile_document(uri).await {
                Ok(result) => {
                    let compiler_diagnostics = compiler_diagnostics_to_lsp(&result.diagnostics, uri);
                    return Ok(compiler_diagnostics);
                }
                Err(e) => {
                    // Don't fail the whole operation if compilation fails
                    warn!("Compilation failed for {}: {}", uri, e);
                }
            }
        }

        // If no compiler is available, return empty diagnostics
        // Tree-sitter is used only for symbol navigation, not error reporting
        Ok(Vec::new())
    }

    /// Get comprehensive diagnostics for all open documents
    pub async fn get_all_document_diagnostics(&mut self) -> HashMap<Url, Vec<Diagnostic>> {
        let mut diagnostics = HashMap::new();
        
        // Collect URIs to avoid borrowing issues
        let uris: Vec<Url> = self.documents.keys().cloned().collect();
        
        for uri in uris {
            match self.get_document_diagnostics(&uri).await {
                Ok(diags) => {
                    if !diags.is_empty() {
                        diagnostics.insert(uri, diags);
                    }
                }
                Err(e) => {
                    warn!("Failed to get diagnostics for {}: {}", uri, e);
                }
            }
        }
        
        diagnostics
    }
}

/// Helper function to convert LSP URI to filesystem path
fn uri_to_path(uri: &Url) -> Result<PathBuf> {
    uri.to_file_path()
        .map_err(|_| anyhow::anyhow!("Failed to convert URI to path: {}", uri))
}

#[derive(Debug, Clone)]
pub struct WorkspaceStats {
    pub document_count: usize,
    pub cache_capacity: usize,
    pub root_uri: Option<Url>,
    pub open_documents: Vec<Url>,
}
