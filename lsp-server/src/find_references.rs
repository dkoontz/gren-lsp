use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tracing::debug;
use crate::symbol_index::SymbolIndex;
use crate::tree_sitter_queries::GrenQueryEngine;
use crate::gren_language;

/// Engine for finding all references to symbols
pub struct FindReferencesEngine {
    /// Symbol index for querying symbols and references
    symbol_index: SymbolIndex,
    /// Tree-sitter query engine for finding symbol usages
    query_engine: GrenQueryEngine,
}

impl FindReferencesEngine {
    /// Create a new find references engine
    pub fn new(symbol_index: SymbolIndex) -> Result<Self> {
        let query_engine = GrenQueryEngine::new()?;
        
        Ok(Self {
            symbol_index,
            query_engine,
        })
    }

    /// Handle textDocument/references LSP request
    pub async fn handle_references(
        &mut self,
        params: ReferenceParams,
        document_content: &str,
    ) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        debug!("🔍 REFERENCES REQUEST: position {:?} in {}, include_declaration: {}", 
               position, uri, include_declaration);
        debug!("🔍 Document content preview: '{}'", &document_content[..std::cmp::min(200, document_content.len())]);

        // Step 1: Find the symbol at the cursor position
        let symbol_at_cursor = match self.find_symbol_at_position(uri, position, document_content).await? {
            Some(symbol) => {
                eprintln!("✅ Symbol found at cursor: '{}' of kind {:?}", symbol.name, symbol.kind);
                symbol
            },
            None => {
                eprintln!("❌ No symbol found at position {:?} in {}", position, uri);
                return Ok(None);
            }
        };

        eprintln!("🔍 Found symbol at cursor: '{}' of kind {:?}", symbol_at_cursor.name, symbol_at_cursor.kind);

        // Step 2: Find all references to this symbol across the workspace
        let references = self.find_all_references_to_symbol(&symbol_at_cursor, include_declaration).await?;

        if references.is_empty() {
            eprintln!("❌ No references found for symbol '{}'", symbol_at_cursor.name);
            Ok(None)
        } else {
            eprintln!("✅ Found {} references for symbol '{}'", references.len(), symbol_at_cursor.name);
            Ok(Some(references))
        }
    }

    /// Find the symbol at a specific position in a document
    async fn find_symbol_at_position(
        &self,
        uri: &Url,
        position: Position,
        document_content: &str,
    ) -> Result<Option<crate::symbol_index::Symbol>> {
        // Parse the document with tree-sitter to find the symbol at the cursor
        let language = gren_language::language()?;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language)?;

        let tree = parser.parse(document_content, None)
            .ok_or_else(|| anyhow!("Failed to parse document for symbol lookup"))?;

        // Convert LSP position to byte offset
        let byte_offset = position_to_byte_offset(document_content, position)?;
        eprintln!("🔍 Position {:?} converted to byte offset: {}", position, byte_offset);
        
        // Find the node at the cursor position
        let root = tree.root_node();
        let node_at_cursor = root.descendant_for_byte_range(byte_offset, byte_offset)
            .ok_or_else(|| anyhow!("No node found at position"))?;
        eprintln!("🔍 Node at cursor: '{}' (kind: {})", get_node_text(node_at_cursor, document_content), node_at_cursor.kind());

        // Find the identifier node (could be parent or self)
        let identifier_node = self.find_identifier_node(node_at_cursor)?;
        let symbol_name = get_node_text(identifier_node, document_content);

        eprintln!("🔍 Symbol name at cursor: '{}' (identifier node kind: {})", symbol_name, identifier_node.kind());

        // Look up the symbol in the symbol index
        let available_symbols = self.symbol_index.find_available_symbols(uri, &symbol_name).await?;
        eprintln!("🔍 Found {} available symbols for '{}'", available_symbols.len(), symbol_name);
        
        // Return the first matching symbol (in Gren, symbols should be unambiguous)
        Ok(available_symbols.into_iter().next())
    }

    /// Find all references to a symbol across the workspace
    async fn find_all_references_to_symbol(
        &self,
        symbol: &crate::symbol_index::Symbol,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        // Find all references to this symbol across the workspace
        let references = self.symbol_index.find_references(&symbol.name).await?;
        eprintln!("🔍 Database returned {} references for symbol '{}'", references.len(), symbol.name);
        let mut all_references = Vec::new();

        for reference in &references {
            eprintln!("🔍 Raw reference: name='{}', kind='{}', uri='{}', range={}:{}-{}:{}", 
                     reference.symbol_name, reference.reference_kind, reference.uri,
                     reference.range_start_line, reference.range_start_char,
                     reference.range_end_line, reference.range_end_char);
            
            // Skip declarations if not requested
            if !include_declaration && (reference.reference_kind == "declaration" || reference.reference_kind == "definition") {
                eprintln!("🔍 Skipping declaration/definition because include_declaration=false");
                continue;
            }

            match reference.to_location() {
                Ok(location) => {
                    eprintln!("🔍 Adding location: {}:{}-{}:{}", 
                             location.range.start.line, location.range.start.character,
                             location.range.end.line, location.range.end.character);
                    all_references.push(location);
                },
                Err(e) => eprintln!("Failed to convert reference to location: {}", e),
            }
        }

        eprintln!("🔍 Final result: {} references for symbol '{}' (include_declaration: {})", 
               all_references.len(), symbol.name, include_declaration);

        Ok(all_references)
    }

    /// Find references to a symbol in a specific file
    async fn find_references_in_file(&self, file_uri: &Url, symbol_name: &str) -> Result<Vec<Location>> {
        debug!("Finding references to '{}' in file {}", symbol_name, file_uri);
        
        let references = self.symbol_index.find_references_in_file(symbol_name, file_uri).await?;
        let mut locations = Vec::new();
        
        for reference in references {
            match reference.to_location() {
                Ok(location) => locations.push(location),
                Err(e) => debug!("Failed to convert reference to location: {}", e),
            }
        }
        
        Ok(locations)
    }

    /// Get all files in the workspace
    async fn get_workspace_files(&self, symbol_index: &SymbolIndex) -> Result<Vec<Url>> {
        // Get all indexed files from the symbol index
        let files = symbol_index.get_indexed_files().await?;
        debug!("Workspace has {} files indexed", files.len());
        Ok(files)
    }

    /// Find the identifier node at or containing the given position
    fn find_identifier_node<'a>(&self, mut node: tree_sitter::Node<'a>) -> Result<tree_sitter::Node<'a>> {
        // Walk up the tree to find an identifier node
        loop {
            let kind = node.kind();
            if kind == "lower_case_identifier" || kind == "upper_case_identifier" {
                return Ok(node);
            }
            
            match node.parent() {
                Some(parent) => node = parent,
                None => return Err(anyhow!("No identifier found at position")),
            }
        }
    }
}

/// Convert LSP position to byte offset in text
fn position_to_byte_offset(text: &str, position: Position) -> Result<usize> {
    let mut line = 0u32;
    let mut character = 0u32;
    let mut byte_offset = 0;

    for ch in text.chars() {
        if line == position.line && character == position.character {
            return Ok(byte_offset);
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }

        byte_offset += ch.len_utf8();
    }

    if line == position.line && character == position.character {
        Ok(byte_offset)
    } else {
        Err(anyhow!("Position {:?} is out of bounds", position))
    }
}

/// Get text content of a tree-sitter node
fn get_node_text(node: tree_sitter::Node, source: &str) -> String {
    node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
}