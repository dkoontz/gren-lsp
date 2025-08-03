use anyhow::Result;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;
use tracing::debug;

use crate::symbol_index::{Symbol, SymbolIndex};

/// Document symbols engine for handling textDocument/documentSymbol requests
pub struct DocumentSymbolsEngine {
    symbol_index: std::sync::Arc<tokio::sync::RwLock<Option<SymbolIndex>>>,
}

impl DocumentSymbolsEngine {
    /// Create a new document symbols engine
    pub fn new(symbol_index: std::sync::Arc<tokio::sync::RwLock<Option<SymbolIndex>>>) -> Self {
        Self { symbol_index }
    }

    /// Handle document symbol request and return hierarchical document symbols
    pub async fn handle_document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        debug!("Handling document symbol request for {}", uri);

        let symbol_index = self.symbol_index.read().await;
        let symbol_index = match symbol_index.as_ref() {
            Some(index) => index,
            None => {
                debug!("Symbol index not initialized");
                return Ok(None);
            }
        };

        // Get all symbols in the file
        let symbols = symbol_index.find_symbols_in_file(uri).await?;
        if symbols.is_empty() {
            debug!("No symbols found in file {}", uri);
            return Ok(None);
        }

        // Build hierarchical structure from flat symbols
        let document_symbols = self.build_document_symbol_hierarchy(symbols)?;

        if document_symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Nested(document_symbols)))
        }
    }

    /// Build hierarchical document symbol structure from flat symbol list
    fn build_document_symbol_hierarchy(&self, symbols: Vec<Symbol>) -> Result<Vec<DocumentSymbol>> {
        let mut document_symbols = Vec::new();
        let mut symbol_map: HashMap<String, DocumentSymbol> = HashMap::new();
        let mut type_constructors: HashMap<String, Vec<DocumentSymbol>> = HashMap::new();

        // Convert symbols to DocumentSymbol and categorize them
        for symbol in symbols {
            let doc_symbol = symbol.to_document_symbol()?;
            let symbol_name = doc_symbol.name.clone();
            
            match doc_symbol.kind {
                // Module level symbols go directly to root
                SymbolKind::MODULE => {
                    document_symbols.push(doc_symbol);
                }
                // Type constructors are children of their parent types
                SymbolKind::CONSTRUCTOR => {
                    if let Some(container) = &symbol.container {
                        type_constructors
                            .entry(container.clone())
                            .or_insert_with(Vec::new)
                            .push(doc_symbol);
                    } else {
                        // Constructor without parent type - add as root level
                        document_symbols.push(doc_symbol);
                    }
                }
                // Other symbols (types, functions, constants) go to the map for hierarchy building
                _ => {
                    symbol_map.insert(symbol_name, doc_symbol);
                }
            }
        }

        // Build hierarchy by checking ranges and nesting patterns
        let mut root_symbols = Vec::new();
        let mut processed_symbols = std::collections::HashSet::new();

        // First pass: identify top-level symbols (those not contained within others)
        for (name, symbol) in &symbol_map {
            if processed_symbols.contains(name) {
                continue;
            }

            let is_top_level = symbol_map.values().all(|other| {
                other.name == symbol.name || !Self::is_range_contained(&symbol.range, &other.range)
            });

            if is_top_level {
                let mut root_symbol = symbol.clone();
                
                // Add type constructors as children if this is a type
                if matches!(symbol.kind, SymbolKind::CLASS | SymbolKind::ENUM | SymbolKind::STRUCT) {
                    if let Some(constructors) = type_constructors.get(&symbol.name) {
                        root_symbol.children = Some(constructors.clone());
                    }
                }

                // Find nested symbols within this symbol's range
                let nested_symbols = self.find_nested_symbols(&symbol, &symbol_map, &processed_symbols);
                if !nested_symbols.is_empty() {
                    let mut children = root_symbol.children.unwrap_or_default();
                    children.extend(nested_symbols);
                    root_symbol.children = Some(children);
                }

                root_symbols.push(root_symbol);
                processed_symbols.insert(name.clone());
                
                // Mark nested symbols as processed
                self.mark_nested_as_processed(&symbol, &symbol_map, &mut processed_symbols);
            }
        }

        // Add remaining unprocessed symbols as root level
        for (name, symbol) in symbol_map {
            if !processed_symbols.contains(&name) {
                let mut root_symbol = symbol;
                
                // Add type constructors as children if this is a type
                if matches!(root_symbol.kind, SymbolKind::CLASS | SymbolKind::ENUM | SymbolKind::STRUCT) {
                    if let Some(constructors) = type_constructors.get(&root_symbol.name) {
                        root_symbol.children = Some(constructors.clone());
                    }
                }
                
                root_symbols.push(root_symbol);
            }
        }

        // Sort symbols by their position in the file
        root_symbols.sort_by(|a, b| {
            a.range.start.line.cmp(&b.range.start.line)
                .then_with(|| a.range.start.character.cmp(&b.range.start.character))
        });

        // Combine with any module-level symbols
        document_symbols.extend(root_symbols);

        Ok(document_symbols)
    }

    /// Check if one range is contained within another
    fn is_range_contained(inner: &Range, outer: &Range) -> bool {
        (outer.start.line < inner.start.line || 
         (outer.start.line == inner.start.line && outer.start.character <= inner.start.character))
        &&
        (outer.end.line > inner.end.line ||
         (outer.end.line == inner.end.line && outer.end.character >= inner.end.character))
    }

    /// Find symbols that are nested within the given parent symbol's range
    fn find_nested_symbols(
        &self,
        parent: &DocumentSymbol,
        symbol_map: &HashMap<String, DocumentSymbol>,
        processed: &std::collections::HashSet<String>,
    ) -> Vec<DocumentSymbol> {
        let mut nested = Vec::new();

        for (name, symbol) in symbol_map {
            if processed.contains(name) || name == &parent.name {
                continue;
            }

            if Self::is_range_contained(&symbol.range, &parent.range) {
                nested.push(symbol.clone());
            }
        }

        // Sort nested symbols by position
        nested.sort_by(|a, b| {
            a.range.start.line.cmp(&b.range.start.line)
                .then_with(|| a.range.start.character.cmp(&b.range.start.character))
        });

        nested
    }

    /// Mark symbols that are nested within the given parent as processed
    fn mark_nested_as_processed(
        &self,
        parent: &DocumentSymbol,
        symbol_map: &HashMap<String, DocumentSymbol>,
        processed: &mut std::collections::HashSet<String>,
    ) {
        for (name, symbol) in symbol_map {
            if name == &parent.name {
                continue;
            }

            if Self::is_range_contained(&symbol.range, &parent.range) {
                processed.insert(name.clone());
            }
        }
    }
}