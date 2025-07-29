use anyhow::{Context, Result};
use tree_sitter::{Parser as TreeSitterParser, Tree, Node, Language};
use std::path::Path;

pub struct Parser {
    parser: TreeSitterParser,
}

impl Parser {
    pub fn new() -> Result<Self> {
        use tracing::info;
        
        info!("Creating new parser");
        let mut parser = TreeSitterParser::new();
        
        info!("Loading Gren grammar");
        // Set Gren language from tree-sitter-gren
        let language = tree_sitter_gren::language();
        
        info!("Setting language for parser");
        parser
            .set_language(language)
            .context("Error loading Gren grammar")?;
        
        info!("Parser created successfully");
        Ok(Self { parser })
    }

    /// Parse a string of Gren source code
    pub fn parse(&mut self, source: &str) -> Result<Option<Tree>> {
        self.parser
            .parse(source, None)
            .context("Failed to parse source code")
            .map(Some)
    }

    /// Parse with incremental updates for better performance
    pub fn parse_incremental(
        &mut self,
        source: &str,
        old_tree: Option<&Tree>,
    ) -> Result<Option<Tree>> {
        self.parser
            .parse(source, old_tree)
            .context("Failed to parse source code incrementally")
            .map(Some)
    }

    /// Parse a file and return the syntax tree
    pub fn parse_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<Tree>> {
        let source = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read file: {}", path.as_ref().display()))?;
        
        self.parse(&source)
    }

    /// Get the Gren language definition
    pub fn language() -> Language {
        tree_sitter_gren::language()
    }

    /// Check if a tree contains syntax errors
    pub fn has_errors(tree: &Tree) -> bool {
        Self::has_node_errors(tree.root_node())
    }

    /// Recursively check for error nodes in the tree
    fn has_node_errors(node: Node) -> bool {
        if node.is_error() || node.is_missing() {
            return true;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if Self::has_node_errors(child) {
                return true;
            }
        }

        false
    }

    /// Extract error information from a tree
    pub fn extract_errors(tree: &Tree) -> Vec<ParseError> {
        let mut errors = Vec::new();
        Self::collect_errors(tree.root_node(), &mut errors);
        errors
    }

    /// Recursively collect all error nodes
    fn collect_errors(node: Node, errors: &mut Vec<ParseError>) {
        if node.is_error() || node.is_missing() {
            errors.push(ParseError {
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_position: node.start_position(),
                end_position: node.end_position(),
                kind: node.kind().to_string(),
                is_missing: node.is_missing(),
            });
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_errors(child, errors);
        }
    }
}

/// Represents a parse error found in the syntax tree
#[derive(Debug, Clone)]
pub struct ParseError {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_position: tree_sitter::Point,
    pub end_position: tree_sitter::Point,
    pub kind: String,
    pub is_missing: bool,
}