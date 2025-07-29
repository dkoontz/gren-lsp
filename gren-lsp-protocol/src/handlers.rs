use gren_lsp_core::{Workspace, Symbol as GrenSymbol};
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::{Error, Result};
use tracing::{debug, info, warn};

pub struct Handlers {
    workspace: Arc<RwLock<Workspace>>,
}

impl Handlers {
    pub fn new(workspace: Arc<RwLock<Workspace>>) -> Self {
        Self { workspace }
    }

    pub async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // TODO: Implement hover
        Ok(None)
    }

    pub async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // TODO: Implement completion
        Ok(None)
    }

    pub async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        // TODO: Implement goto definition
        Ok(None)
    }

    pub async fn find_references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        // TODO: Implement find references
        Ok(None)
    }

    pub async fn document_symbols(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        info!("Document symbols requested for: {}", params.text_document.uri);
        
        let workspace = self.workspace.read().await;
        
        // Get symbols for the specific file
        match workspace.get_file_symbols(&params.text_document.uri) {
            Ok(symbols) => {
                if symbols.is_empty() {
                    debug!("No symbols found for document: {}", params.text_document.uri);
                    return Ok(None);
                }
                
                info!("Found {} symbols for document: {}", symbols.len(), params.text_document.uri);
                
                // Convert to LSP document symbols with hierarchy
                let document_symbols = self.convert_to_document_symbols(symbols);
                
                Ok(Some(DocumentSymbolResponse::Nested(document_symbols)))
            }
            Err(e) => {
                warn!("Failed to get symbols for document {}: {}", params.text_document.uri, e);
                Ok(None)
            }
        }
    }

    pub async fn workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        info!("Workspace symbols requested with query: '{}'", params.query);
        
        let workspace = self.workspace.read().await;
        
        // Search for symbols matching the query
        match workspace.find_symbols(&params.query) {
            Ok(symbols) => {
                if symbols.is_empty() {
                    debug!("No symbols found for query: '{}'", params.query);
                    return Ok(Some(Vec::new()));
                }
                
                info!("Found {} symbols for query: '{}'", symbols.len(), params.query);
                
                // Convert to LSP symbol information
                let symbol_information = symbols
                    .into_iter()
                    .map(|symbol| self.convert_to_symbol_information(symbol))
                    .collect();
                
                Ok(Some(symbol_information))
            }
            Err(e) => {
                warn!("Failed to search symbols for query '{}': {}", params.query, e);
                Ok(Some(Vec::new()))
            }
        }
    }

    pub async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        // TODO: Implement code actions
        Ok(None)
    }

    pub async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        // TODO: Implement rename
        Ok(None)
    }

    // Helper methods for symbol conversion

    /// Convert internal symbols to LSP DocumentSymbol format with hierarchy
    fn convert_to_document_symbols(&self, symbols: Vec<GrenSymbol>) -> Vec<DocumentSymbol> {
        let mut document_symbols = Vec::new();
        let mut processed_modules = std::collections::HashSet::new();
        let mut processed_types = std::collections::HashSet::new();

        // Group symbols by type for hierarchical organization
        let mut modules = Vec::new();
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut constructors = Vec::new();

        for symbol in &symbols {
            match symbol.kind {
                SymbolKind::MODULE => modules.push(symbol.clone()),
                SymbolKind::CLASS => types.push(symbol.clone()),
                SymbolKind::FUNCTION => functions.push(symbol.clone()),
                SymbolKind::CONSTRUCTOR => constructors.push(symbol.clone()),
                _ => {}
            }
        }

        // Add modules first (top-level)
        for module in modules {
            // Skip modules that are just file names or duplicates
            if module.name.contains(" exposing ") || processed_modules.contains(&module.name) {
                continue;
            }
            
            let doc_symbol = DocumentSymbol {
                name: module.name.clone(),
                detail: None,
                kind: module.kind,
                range: module.location.range,
                selection_range: module.location.range,
                children: None,
                tags: None,
                deprecated: None,
            };
            document_symbols.push(doc_symbol);
            processed_modules.insert(module.name.clone());
        }

        // Sort types to process simple names first, then verbose ones
        types.sort_by(|a, b| {
            let a_is_verbose = a.name.starts_with("type ");
            let b_is_verbose = b.name.starts_with("type ");
            a_is_verbose.cmp(&b_is_verbose)
        });
        
        // Process types with smart deduplication
        for typ in types {
            // Extract clean type name
            let type_name = if typ.name.starts_with("type ") {
                // Extract type name from "type Foo = ..." or "type alias Foo = ..."
                if let Some(name_part) = typ.name.split_whitespace().nth(1) {
                    name_part.split('=').next().unwrap_or(name_part).trim().to_string()
                } else {
                    continue;
                }
            } else {
                typ.name.clone()
            };
            
            // Skip if we already processed this type name
            if processed_types.contains(&type_name) {
                continue;
            }
            
            // Find constructors that belong to this type using container_name
            let type_constructors: Vec<DocumentSymbol> = constructors
                .iter()
                .filter(|c| {
                    // Use the container_name field to properly associate constructors with their parent type
                    c.container_name.as_ref().map(|container| container == &type_name).unwrap_or(false)
                })
                .map(|c| DocumentSymbol {
                    name: c.name.clone(),
                    detail: c.type_signature.clone(),
                    kind: c.kind,
                    range: c.location.range,
                    selection_range: c.location.range,
                    children: None,
                    tags: None,
                    deprecated: None,
                })
                .collect();

            let doc_symbol = DocumentSymbol {
                name: type_name.clone(),
                detail: typ.type_signature.clone(),
                kind: typ.kind,
                range: typ.location.range,
                selection_range: typ.location.range,
                children: if type_constructors.is_empty() {
                    None
                } else {
                    Some(type_constructors)
                },
                tags: None,
                deprecated: None,
            };
            
            document_symbols.push(doc_symbol);
            processed_types.insert(type_name);
        }

        // Add functions (no deduplication needed as they're typically unique)
        for function in functions {
            let doc_symbol = DocumentSymbol {
                name: function.name.clone(),
                detail: function.type_signature.clone(),
                kind: function.kind,
                range: function.location.range,
                selection_range: function.location.range,
                children: None,
                tags: None,
                deprecated: None,
            };
            document_symbols.push(doc_symbol);
        }

        // Sort by line number for consistent ordering
        document_symbols.sort_by(|a, b| a.range.start.line.cmp(&b.range.start.line));

        debug!("Converted {} internal symbols to {} document symbols", 
               symbols.len(), document_symbols.len());

        document_symbols
    }

    /// Convert internal symbol to LSP SymbolInformation format
    fn convert_to_symbol_information(&self, symbol: GrenSymbol) -> SymbolInformation {
        let container_name = symbol.container_name.clone();
        
        SymbolInformation {
            name: symbol.name,
            kind: symbol.kind,
            location: symbol.location,
            container_name,
            tags: None,
            deprecated: None,
        }
    }
}