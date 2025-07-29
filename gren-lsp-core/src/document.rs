use crate::{Parser, ParseError};
use anyhow::Result;
use lsp_textdocument::FullTextDocument;
use lsp_types::*;
use std::time::Instant;
use tree_sitter::Tree;

pub struct Document {
    text_document: FullTextDocument,
    uri: Url,
    parse_tree: Option<Tree>,
    last_parsed: Option<Instant>,
    parse_errors: Vec<ParseError>,
}

impl Document {
    pub fn new(text_document_item: TextDocumentItem) -> Self {
        let uri = text_document_item.uri.clone();
        let text_document = FullTextDocument::new(
            text_document_item.language_id,
            text_document_item.version,
            text_document_item.text,
        );

        Self { 
            text_document, 
            uri,
            parse_tree: None,
            last_parsed: None,
            parse_errors: Vec::new(),
        }
    }

    pub fn uri(&self) -> &Url {
        &self.uri
    }

    pub fn version(&self) -> i32 {
        self.text_document.version()
    }

    pub fn text(&self) -> &str {
        self.text_document.get_content(None)
    }

    pub fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        let new_version = self.version() + 1;
        self.text_document.update(&changes, new_version);
        
        // Invalidate parse tree when content changes
        self.invalidate_parse_tree();
        
        Ok(())
    }

    pub fn position_to_offset(&self, position: Position) -> Option<usize> {
        Some(self.text_document.offset_at(position) as usize)
    }

    pub fn offset_to_position(&self, offset: usize) -> Option<Position> {
        Some(self.text_document.position_at(offset as u32))
    }

    /// Get the current parse tree, parsing if necessary
    pub fn get_parse_tree(&mut self, parser: &mut Parser) -> Result<Option<&Tree>> {
        use tracing::info;
        
        let has_tree = self.parse_tree.is_some();
        let needs_reparse = self.needs_reparse();
        info!("get_parse_tree: has_tree={}, needs_reparse={}", has_tree, needs_reparse);
        
        if self.parse_tree.is_none() || needs_reparse {
            info!("Triggering reparse...");
            self.reparse(parser)?;
        } else {
            info!("Using existing parse tree");
        }
        Ok(self.parse_tree.as_ref())
    }

    /// Force a reparse of the document
    pub fn reparse(&mut self, parser: &mut Parser) -> Result<()> {
        use tracing::info;
        
        let source = self.text();
        info!("Reparsing document, source length: {} bytes", source.len());
        
        // TEMPORARY: Always use full parsing to avoid incremental parsing issues
        // TODO: Fix incremental parsing later for better performance
        info!("Using full parsing (incremental parsing temporarily disabled)");
        let new_tree = parser.parse(source)?;

        if let Some(tree) = new_tree {
            // Extract parse errors with source text for better error messages
            self.parse_errors = Parser::extract_errors_with_source(&tree, source);
            info!("Parse completed: {} errors found", self.parse_errors.len());
            
            self.parse_tree = Some(tree);
            self.last_parsed = Some(Instant::now());
        } else {
            info!("Parse failed - no tree returned");
        }

        Ok(())
    }

    /// Invalidate the parse tree (called when document changes)
    fn invalidate_parse_tree(&mut self) {
        // Keep the old tree for incremental parsing, but mark as needing reparse
        self.last_parsed = None;
    }

    /// Check if document needs reparsing
    fn needs_reparse(&self) -> bool {
        let needs_reparse = self.last_parsed.is_none();
        use tracing::info;
        info!("Document needs reparse: {}", needs_reparse);
        needs_reparse
    }

    /// Get parse errors from the last parse
    pub fn parse_errors(&self) -> &[ParseError] {
        &self.parse_errors
    }

    /// Check if document has syntax errors
    pub fn has_syntax_errors(&self) -> bool {
        !self.parse_errors.is_empty()
    }

    /// Get the language ID of the document
    pub fn language_id(&self) -> &str {
        self.text_document.language_id()
    }

    /// Get the size in bytes of the document
    pub fn size(&self) -> usize {
        self.text().len()
    }

    /// Get the last modification time (version)
    pub fn last_modified(&self) -> i32 {
        self.version()
    }
}