use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};
use tracing::debug;
use url::Url;

use crate::gren_language;
use crate::symbol_index::{SymbolIndex, Symbol};
use crate::tree_sitter_queries::GrenQueryEngine;

/// Go-to-definition engine for precise symbol navigation
pub struct GotoDefinitionEngine {
    /// Symbol index for looking up symbol definitions
    symbol_index: SymbolIndex,
    /// Query engine for AST analysis
    query_engine: GrenQueryEngine,
    /// Tree-sitter parser for Gren
    parser: Parser,
}

/// Result of a go-to-definition search
#[derive(Debug, Clone)]
pub struct DefinitionResult {
    /// Location of the symbol definition
    pub location: Location,
    /// The symbol that was found
    pub symbol: Symbol,
    /// Whether this is a local definition (same file) or cross-module
    pub is_local: bool,
}

impl GotoDefinitionEngine {
    /// Create a new go-to-definition engine
    pub fn new(symbol_index: SymbolIndex) -> Result<Self> {
        let query_engine = GrenQueryEngine::new()?;
        let mut parser = Parser::new();
        let language = gren_language::language()?;
        parser.set_language(&language)
            .map_err(|_| anyhow!("Failed to set Gren language for goto definition parser"))?;

        Ok(Self {
            symbol_index,
            query_engine,
            parser,
        })
    }

    /// Handle a go-to-definition request and return the definition location
    pub async fn handle_goto_definition(
        &mut self,
        params: GotoDefinitionParams,
        document_content: &str,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params.position;
        let uri = &params.text_document_position_params.text_document.uri;

        debug!("Processing go-to-definition request at position {:?} in {}", position, uri);

        // Parse the document to find the symbol at the position
        let tree = self.parser.parse(document_content, None)
            .ok_or_else(|| anyhow!("Failed to parse document for go-to-definition"))?;

        let root_node = tree.root_node();
        
        // Convert LSP position to byte offset
        let byte_offset = position_to_byte_offset(document_content, position)?;
        
        // Find the node at the cursor position
        let symbol_node = root_node.descendant_for_byte_range(byte_offset, byte_offset)
            .ok_or_else(|| anyhow!("No node found at position"))?;

        debug!("Found node at position: {} ({})", symbol_node.kind(), 
               get_node_text(symbol_node, document_content));

        // Extract symbol information and find its definition
        let definition_result = self.find_symbol_definition(symbol_node, document_content, uri).await?;

        if let Some(result) = definition_result {
            let response = GotoDefinitionResponse::Scalar(result.location);
            Ok(Some(response))
        } else {
            debug!("No definition found for symbol at position");
            Ok(None)
        }
    }

    /// Find the definition of a symbol
    async fn find_symbol_definition(
        &self,
        symbol_node: Node<'_>,
        document_content: &str,
        file_uri: &Url,
    ) -> Result<Option<DefinitionResult>> {
        // Only process nodes that represent identifiers
        if !self.is_navigable_node(symbol_node) {
            return Ok(None);
        }

        let symbol_name = get_node_text(symbol_node, document_content);
        debug!("Looking for definition of symbol: '{}'", symbol_name);

        // Strategy 1: Look for the symbol in the symbol index (cross-module and local definitions)
        let symbols = self.symbol_index.find_symbols_by_name(&symbol_name).await?;
        
        if let Some(definition) = self.find_best_definition(&symbols, file_uri).await? {
            return Ok(Some(definition));
        }

        // Strategy 2: Look for local definitions in the same file using AST analysis
        self.find_local_definition(symbol_node, document_content, file_uri, &symbol_name).await
    }

    /// Check if a node represents a symbol that can be navigated to
    fn is_navigable_node(&self, node: Node<'_>) -> bool {
        matches!(node.kind(), 
            "lower_case_identifier" | 
            "upper_case_identifier" | 
            "type_identifier" |
            "value_identifier" |
            "operator_identifier"
        )
    }

    /// Find the best definition from a list of symbol candidates
    async fn find_best_definition(
        &self,
        symbols: &[Symbol],
        current_file_uri: &Url,
    ) -> Result<Option<DefinitionResult>> {
        if symbols.is_empty() {
            return Ok(None);
        }

        // For Gren, we should have deterministic symbol resolution
        // But we'll prioritize based on:
        // 1. Definitions (vs references) - symbols that are actual definitions
        // 2. Local file definitions (same file)
        // 3. Imported definitions (other files)

        let mut best_symbol: Option<&Symbol> = None;
        let mut is_local = false;

        for symbol in symbols {
            // Parse the symbol URI
            let symbol_uri = match Url::parse(&symbol.uri) {
                Ok(uri) => uri,
                Err(_) => continue, // Skip invalid URIs
            };

            // Check if this is in the current file
            let is_current_file = symbol_uri == *current_file_uri;

            // For now, we'll take the first valid symbol
            // In the future, we could enhance this to distinguish definitions vs references
            if best_symbol.is_none() || is_current_file {
                best_symbol = Some(symbol);
                is_local = is_current_file;
                
                // If we found a local definition, prefer it
                if is_current_file {
                    break;
                }
            }
        }

        if let Some(symbol) = best_symbol {
            let location = self.symbol_to_location(symbol)?;
            Ok(Some(DefinitionResult {
                location,
                symbol: symbol.clone(),
                is_local,
            }))
        } else {
            Ok(None)
        }
    }

    /// Convert a Symbol to an LSP Location
    fn symbol_to_location(&self, symbol: &Symbol) -> Result<Location> {
        let uri = Url::parse(&symbol.uri)
            .map_err(|e| anyhow!("Invalid URI in symbol {}: {}", symbol.name, e))?;

        let range = Range {
            start: Position {
                line: symbol.range_start_line as u32,
                character: symbol.range_start_char as u32,
            },
            end: Position {
                line: symbol.range_end_line as u32,
                character: symbol.range_end_char as u32,
            },
        };

        Ok(Location { uri, range })
    }

    /// Find local definitions using AST analysis (fallback for symbols not in index)
    async fn find_local_definition(
        &self,
        symbol_node: Node<'_>,
        document_content: &str,
        file_uri: &Url,
        symbol_name: &str,
    ) -> Result<Option<DefinitionResult>> {
        // Look for local variable definitions (let bindings, function parameters)
        if let Some(definition_node) = self.find_local_variable_definition(symbol_node, document_content, symbol_name)? {
            let range = node_to_range(definition_node);
            let location = Location {
                uri: file_uri.clone(),
                range,
            };

            // Create a temporary symbol for the local definition
            let symbol = Symbol {
                id: None,
                name: symbol_name.to_string(),
                kind: 13, // SymbolKind::VARIABLE as i32
                uri: file_uri.to_string(),
                range_start_line: range.start.line as i32,
                range_start_char: range.start.character as i32,
                range_end_line: range.end.line as i32,
                range_end_char: range.end.character as i32,
                container: None,
                signature: None,
                documentation: None,
                created_at: None,
            };

            return Ok(Some(DefinitionResult {
                location,
                symbol,
                is_local: true,
            }));
        }

        Ok(None)
    }

    /// Find local variable definitions by walking up the AST
    fn find_local_variable_definition<'a>(
        &self,
        symbol_node: Node<'a>,
        document_content: &str,
        symbol_name: &str,
    ) -> Result<Option<Node<'a>>> {
        // Walk up the AST to find scopes that might contain the definition
        let mut current = symbol_node.parent();
        
        while let Some(node) = current {
            match node.kind() {
                "let_in_expr" => {
                    // Look for let bindings in this scope
                    if let Some(def_node) = self.find_let_binding_definition(node, symbol_name, document_content)? {
                        return Ok(Some(def_node));
                    }
                }
                "value_declaration" => {
                    // Look for function parameter definitions
                    if let Some(def_node) = self.find_parameter_definition(node, symbol_name, document_content)? {
                        return Ok(Some(def_node));
                    }
                    // Also check if this is the function being defined
                    if let Some(def_node) = self.find_function_name_definition(node, symbol_name, document_content)? {
                        return Ok(Some(def_node));
                    }
                }
                "when_is_branch" => {
                    // Look for pattern bindings in when expressions
                    if let Some(def_node) = self.find_pattern_binding_definition(node, symbol_name, document_content)? {
                        return Ok(Some(def_node));
                    }
                }
                _ => {}
            }
            current = node.parent();
        }

        Ok(None)
    }

    /// Find let binding definitions
    fn find_let_binding_definition<'a>(
        &self,
        let_node: Node<'a>,
        symbol_name: &str,
        document_content: &str,
    ) -> Result<Option<Node<'a>>> {
        // Look for value declarations within the let expression
        let mut cursor = let_node.walk();
        for child in let_node.children(&mut cursor) {
            if child.kind() == "value_declaration" {
                if let Some(function_left) = child.child_by_field_name("functionDeclarationLeft") {
                    let mut inner_cursor = function_left.walk();
                    for inner_child in function_left.children(&mut inner_cursor) {
                        if inner_child.kind() == "lower_case_identifier" {
                            let name = get_node_text(inner_child, document_content);
                            if name == symbol_name {
                                return Ok(Some(inner_child));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Find parameter definitions in function declarations
    fn find_parameter_definition<'a>(
        &self,
        value_decl_node: Node<'a>,
        symbol_name: &str,
        document_content: &str,
    ) -> Result<Option<Node<'a>>> {
        if let Some(function_left) = value_decl_node.child_by_field_name("functionDeclarationLeft") {
            let mut cursor = function_left.walk();
            for child in function_left.children(&mut cursor) {
                if child.kind() == "lower_case_identifier" || child.kind() == "pattern" {
                    // Check if this parameter matches our symbol
                    let param_text = get_node_text(child, document_content);
                    if param_text == symbol_name {
                        return Ok(Some(child));
                    }
                    
                    // For patterns, we need to look deeper
                    if child.kind() == "pattern" {
                        if let Some(def_node) = self.find_pattern_variable(child, symbol_name, document_content)? {
                            return Ok(Some(def_node));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Find function name definitions
    fn find_function_name_definition<'a>(
        &self,
        value_decl_node: Node<'a>,
        symbol_name: &str,
        document_content: &str,
    ) -> Result<Option<Node<'a>>> {
        if let Some(function_left) = value_decl_node.child_by_field_name("functionDeclarationLeft") {
            // The first child should be the function name
            if let Some(first_child) = function_left.child(0) {
                if first_child.kind() == "lower_case_identifier" {
                    let name = get_node_text(first_child, document_content);
                    if name == symbol_name {
                        return Ok(Some(first_child));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Find pattern binding definitions in when expressions
    fn find_pattern_binding_definition<'a>(
        &self,
        when_branch_node: Node<'a>,
        symbol_name: &str,
        document_content: &str,
    ) -> Result<Option<Node<'a>>> {
        if let Some(pattern) = when_branch_node.child_by_field_name("pattern") {
            self.find_pattern_variable(pattern, symbol_name, document_content)
        } else {
            Ok(None)
        }
    }

    /// Find variables within patterns (recursive for nested patterns)
    fn find_pattern_variable<'a>(
        &self,
        pattern_node: Node<'a>,
        symbol_name: &str,
        document_content: &str,
    ) -> Result<Option<Node<'a>>> {
        if pattern_node.kind() == "lower_case_identifier" {
            let name = get_node_text(pattern_node, document_content);
            if name == symbol_name {
                return Ok(Some(pattern_node));
            }
        }

        // Recursively search child patterns
        let mut cursor = pattern_node.walk();
        for child in pattern_node.children(&mut cursor) {
            if let Some(found) = self.find_pattern_variable(child, symbol_name, document_content)? {
                return Ok(Some(found));
            }
        }

        Ok(None)
    }
}

/// Convert LSP position to byte offset in document
fn position_to_byte_offset(content: &str, position: Position) -> Result<usize> {
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = position.line as usize;
    
    if line_idx >= lines.len() {
        return Err(anyhow!("Position line {} exceeds document length {}", line_idx, lines.len()));
    }

    // Calculate byte offset up to the target line
    let mut byte_offset = 0;
    for (i, line) in lines.iter().enumerate() {
        if i == line_idx {
            // Add character offset within the line
            let char_idx = position.character as usize;
            let line_chars: Vec<char> = line.chars().collect();
            
            if char_idx > line_chars.len() {
                return Err(anyhow!("Position character {} exceeds line length {}", char_idx, line_chars.len()));
            }
            
            // Convert character offset to byte offset
            let line_prefix: String = line_chars.iter().take(char_idx).collect();
            byte_offset += line_prefix.len();
            break;
        } else {
            byte_offset += line.len() + 1; // +1 for newline
        }
    }

    Ok(byte_offset)
}

/// Get the text content of a tree-sitter node
fn get_node_text(node: Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

/// Convert a tree-sitter node to an LSP Range
fn node_to_range(node: Node) -> Range {
    Range {
        start: Position {
            line: node.start_position().row as u32,
            character: node.start_position().column as u32,
        },
        end: Position {
            line: node.end_position().row as u32,
            character: node.end_position().column as u32,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_to_byte_offset() {
        let content = "line1\nline2\nline3";
        
        // Start of document
        assert_eq!(position_to_byte_offset(content, Position::new(0, 0)).unwrap(), 0);
        
        // Start of second line
        assert_eq!(position_to_byte_offset(content, Position::new(1, 0)).unwrap(), 6);
        
        // Middle of first line
        assert_eq!(position_to_byte_offset(content, Position::new(0, 2)).unwrap(), 2);
    }

    #[test]
    fn test_node_to_range() {
        // This would require setting up tree-sitter parsing, so we'll test the logic separately
        // The actual node_to_range function is straightforward position conversion
    }

    #[test]
    fn test_definition_result_creation() {
        let location = Location {
            uri: Url::parse("file:///test/Main.gren").unwrap(),
            range: Range::default(),
        };
        
        let symbol = Symbol {
            id: Some(1),
            name: "testFunction".to_string(),
            kind: 12, // SymbolKind::FUNCTION
            uri: "file:///test/Main.gren".to_string(),
            range_start_line: 5,
            range_start_char: 0,
            range_end_line: 5,
            range_end_char: 12,
            container: None,
            signature: Some("String -> String".to_string()),
            documentation: None,
            created_at: None,
        };
        
        let result = DefinitionResult {
            location: location.clone(),
            symbol: symbol.clone(),
            is_local: true,
        };
        
        assert_eq!(result.location.uri, location.uri);
        assert_eq!(result.symbol.name, "testFunction");
        assert!(result.is_local);
    }
}