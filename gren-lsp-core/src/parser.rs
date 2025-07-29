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
        Self::collect_errors(tree.root_node(), None, &mut errors);
        errors
    }

    /// Extract error information from a tree with source text for better messages
    pub fn extract_errors_with_source(tree: &Tree, source: &str) -> Vec<ParseError> {
        let mut errors = Vec::new();
        Self::collect_errors(tree.root_node(), Some(source.as_bytes()), &mut errors);
        
        // TODO: In the future, we could add semantic validation here by:
        // 1. Building up type information from the tree-sitter parse
        // 2. Tracking type constructor definitions and their arities 
        // 3. Validating type applications against known constructors
        // This would allow us to catch errors like missing arrows in type signatures
        // without making unfounded assumptions about the type system.
        
        errors
    }

    /// Recursively collect all error nodes
    fn collect_errors(node: Node, source: Option<&[u8]>, errors: &mut Vec<ParseError>) {
        if node.is_error() || node.is_missing() {
            let context = Self::get_error_context(node, source);
            errors.push(ParseError {
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_position: node.start_position(),
                end_position: node.end_position(),
                kind: node.kind().to_string(),
                is_missing: node.is_missing(),
                context,
            });
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_errors(child, source, errors);
        }
    }


    /// Get additional context for error nodes
    fn get_error_context(node: Node, source: Option<&[u8]>) -> ParseErrorContext {
        let mut context = ParseErrorContext::default();
        
        // Get parent context for better error messages
        if let Some(parent) = node.parent() {
            context.parent_kind = Some(parent.kind().to_string());
            
            // Try to infer what was expected based on parent context
            context.expected = Self::infer_expected_token(&parent, node);
        }
        
        // Get the actual text content if available
        if let Some(source_bytes) = source {
            if let Ok(source_text) = node.utf8_text(source_bytes) {
                context.actual_text = Some(source_text.to_string());
            }
        }
        
        // Get sibling context
        if let Some(prev_sibling) = node.prev_sibling() {
            context.previous_sibling = Some(prev_sibling.kind().to_string());
        }
        if let Some(next_sibling) = node.next_sibling() {
            context.next_sibling = Some(next_sibling.kind().to_string());
        }
        
        context
    }
    
    /// Try to infer what token was expected based on context
    fn infer_expected_token(parent: &Node, error_node: Node) -> Option<String> {
        match parent.kind() {
            "type_annotation" => {
                // In type annotations, we can infer based on position
                // Common pattern: "identifier : Type -> ReturnType"
                let error_start = error_node.start_byte();
                let parent_children: Vec<_> = parent.children(&mut parent.walk()).collect();
                
                // Find where the error is in relation to other children
                for (i, child) in parent_children.iter().enumerate() {
                    if child.start_byte() >= error_start {
                        return match i {
                            1 => Some("':'".to_string()),
                            2 => Some("'->'".to_string()),
                            3 => Some("type".to_string()),
                            _ => Some("type annotation".to_string()),
                        };
                    }
                }
                
                Some("'->' or type".to_string())
            }
            "function_declaration" => {
                Some("'=' or function body".to_string())
            }
            "let_expression" => {
                Some("'in' keyword".to_string())
            }
            "comment" => {
                Some("comment content".to_string())
            }
            _ => None
        }
    }
}

/// Additional context for parse errors
#[derive(Debug, Clone, Default)]
pub struct ParseErrorContext {
    pub parent_kind: Option<String>,
    pub expected: Option<String>,
    pub actual_text: Option<String>,
    pub previous_sibling: Option<String>,
    pub next_sibling: Option<String>,
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
    pub context: ParseErrorContext,
}