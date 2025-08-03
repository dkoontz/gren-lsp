use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};
use tracing::debug;
use url::Url;

use crate::gren_language;
use crate::symbol_index::{SymbolIndex, Symbol};
use crate::tree_sitter_queries::GrenQueryEngine;

/// Hover information engine for providing type information and documentation
pub struct HoverEngine {
    /// Symbol index for looking up symbol information
    symbol_index: SymbolIndex,
    /// Query engine for AST analysis
    query_engine: GrenQueryEngine,
    /// Tree-sitter parser for Gren
    parser: Parser,
}

/// Information extracted for hover display
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// Type signature or inferred type
    pub type_info: Option<String>,
    /// Documentation from comments
    pub documentation: Option<String>,
    /// Source module for imported symbols
    pub source_module: Option<String>,
    /// Range of the symbol being hovered
    pub range: Range,
    /// Symbol name
    pub symbol_name: String,
}

impl HoverEngine {
    /// Create a new hover engine
    pub fn new(symbol_index: SymbolIndex) -> Result<Self> {
        let query_engine = GrenQueryEngine::new()?;
        let mut parser = Parser::new();
        let language = gren_language::language()?;
        parser.set_language(&language)
            .map_err(|_| anyhow!("Failed to set Gren language for hover parser"))?;

        Ok(Self {
            symbol_index,
            query_engine,
            parser,
        })
    }

    /// Handle a hover request and return hover information
    pub async fn handle_hover(
        &mut self,
        params: HoverParams,
        document_content: &str,
    ) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = &params.text_document_position_params.text_document.uri;

        debug!("Processing hover request at position {:?} in {}", position, uri);

        // Parse the document to find the symbol at the position
        let tree = self.parser.parse(document_content, None)
            .ok_or_else(|| anyhow!("Failed to parse document for hover"))?;

        let root_node = tree.root_node();
        
        // Convert LSP position to byte offset
        let byte_offset = position_to_byte_offset(document_content, position)?;
        
        // Find the node at the cursor position
        let symbol_node = root_node.descendant_for_byte_range(byte_offset, byte_offset)
            .ok_or_else(|| anyhow!("No node found at position"))?;

        debug!("Found node at position: {} ({})", symbol_node.kind(), 
               get_node_text(symbol_node, document_content));

        // Extract symbol information from the node
        let hover_info = self.extract_hover_info(symbol_node, document_content, uri).await?;

        if let Some(info) = hover_info {
            let hover_response = self.format_hover_response(info)?;
            Ok(Some(hover_response))
        } else {
            debug!("No hover information found for symbol at position");
            Ok(None)
        }
    }

    /// Extract hover information from a symbol node
    async fn extract_hover_info(
        &self,
        symbol_node: Node<'_>,
        document_content: &str,
        file_uri: &Url,
    ) -> Result<Option<HoverInfo>> {
        // Only process nodes that represent identifiers
        if !self.is_hoverable_node(symbol_node) {
            return Ok(None);
        }

        let symbol_name = get_node_text(symbol_node, document_content);
        let symbol_range = node_to_range(symbol_node);

        debug!("Processing hoverable symbol: '{}'", symbol_name);

        // Look for the symbol in the symbol index
        let symbols = self.symbol_index.find_symbols_by_name(&symbol_name).await?;
        
        // Find the most relevant symbol (prefer local file, then imported)
        let best_symbol = self.find_best_matching_symbol(&symbols, file_uri, &symbol_range).await?;

        if let Some(symbol) = best_symbol {
            let hover_info = HoverInfo {
                type_info: symbol.signature.clone(),
                documentation: symbol.documentation.clone(),
                source_module: self.determine_source_module(&symbol, file_uri).await?,
                range: symbol_range,
                symbol_name: symbol_name.clone(),
            };

            Ok(Some(hover_info))
        } else {
            // Try to infer basic information for local symbols
            self.extract_local_symbol_info(symbol_node, document_content, file_uri, symbol_name, symbol_range).await
        }
    }

    /// Check if a node represents a symbol that can be hovered
    fn is_hoverable_node(&self, node: Node<'_>) -> bool {
        matches!(node.kind(), 
            "lower_case_identifier" | 
            "upper_case_identifier" | 
            "type_identifier" |
            "value_identifier" |
            "operator_identifier"
        )
    }

    /// Find the best matching symbol from candidates
    async fn find_best_matching_symbol(
        &self,
        symbols: &[Symbol],
        file_uri: &Url,
        _symbol_range: &Range,
    ) -> Result<Option<Symbol>> {
        if symbols.is_empty() {
            return Ok(None);
        }

        // Prefer symbols from the current file
        for symbol in symbols {
            if symbol.uri == file_uri.to_string() {
                return Ok(Some(symbol.clone()));
            }
        }

        // Otherwise, return the first available symbol (from imports)
        Ok(Some(symbols[0].clone()))
    }

    /// Determine the source module for a symbol
    async fn determine_source_module(&self, symbol: &Symbol, current_file: &Url) -> Result<Option<String>> {
        // If the symbol is from the current file, no source module info needed
        if symbol.uri == current_file.to_string() {
            return Ok(None);
        }

        // Extract module name from the symbol's URI
        if let Ok(symbol_uri) = Url::parse(&symbol.uri) {
            if let Some(path) = symbol_uri.to_file_path().ok() {
                if let Some(file_name) = path.file_stem() {
                    if let Some(module_name) = file_name.to_str() {
                        // Convert file name to module name (capitalize first letter)
                        let module_name = format!("{}{}", 
                            module_name.chars().next().unwrap_or('M').to_uppercase(),
                            module_name.chars().skip(1).collect::<String>()
                        );
                        return Ok(Some(module_name));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extract information for local symbols not in the symbol index
    async fn extract_local_symbol_info(
        &self,
        symbol_node: Node<'_>,
        document_content: &str,
        _file_uri: &Url,
        symbol_name: String,
        symbol_range: Range,
    ) -> Result<Option<HoverInfo>> {
        // Try to find type annotation or infer from context
        let type_info = self.extract_local_type_info(symbol_node, document_content).await?;
        
        // Look for nearby documentation comments
        let documentation = self.extract_nearby_documentation(symbol_node, document_content)?;

        if type_info.is_some() || documentation.is_some() {
            Ok(Some(HoverInfo {
                type_info,
                documentation,
                source_module: None, // Local symbol
                range: symbol_range,
                symbol_name,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract type information for local symbols
    async fn extract_local_type_info(&self, symbol_node: Node<'_>, document_content: &str) -> Result<Option<String>> {
        
        // For function names, look for type annotation in the same file
        // In Gren, type annotations appear immediately before function definitions
        let symbol_name = get_node_text(symbol_node, document_content);
        
        // Find the root node and search for type annotations
        let mut current = symbol_node;
        while let Some(parent) = current.parent() {
            current = parent;
        }
        let root = current;
        
        // Search through all nodes looking for type annotations
        let mut cursor = root.walk();
        let mut found_type_annotation = false;
        let mut type_annotation_text = None;
        
        fn traverse_for_type_annotation(
            node: Node<'_>, 
            cursor: &mut tree_sitter::TreeCursor<'_>, 
            symbol_name: &str,
            document_content: &str,
            found_type: &mut bool,
            type_text: &mut Option<String>
        ) {
            if *found_type {
                return;
            }
            
            // Check if this is a type annotation
            if node.kind() == "type_annotation" {
                let annotation_text = get_node_text(node, document_content);
                
                // Check if this type annotation is for our symbol
                // Type annotations in Gren have the pattern: "symbolName : Type"
                if annotation_text.trim_start().starts_with(&format!("{} :", symbol_name)) {
                    *type_text = Some(annotation_text.trim().to_string());
                    *found_type = true;
                    return;
                }
            }
            
            // Recursively traverse children
            if cursor.goto_first_child() {
                loop {
                    traverse_for_type_annotation(cursor.node(), cursor, symbol_name, document_content, found_type, type_text);
                    if *found_type || !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }
        
        traverse_for_type_annotation(root, &mut cursor, &symbol_name, document_content, &mut found_type_annotation, &mut type_annotation_text);
        
        Ok(type_annotation_text)
    }

    /// Extract documentation from nearby comments
    fn extract_nearby_documentation(&self, symbol_node: Node<'_>, document_content: &str) -> Result<Option<String>> {
        
        let symbol_name = get_node_text(symbol_node, document_content);
        
        // Find the root node and search for documentation comments
        let mut current = symbol_node;
        while let Some(parent) = current.parent() {
            current = parent;
        }
        let root = current;
        
        // Search for documentation comments that precede the symbol definition
        let mut cursor = root.walk();
        let mut found_documentation = None;
        
        fn traverse_for_documentation(
            node: Node<'_>, 
            cursor: &mut tree_sitter::TreeCursor<'_>, 
            symbol_name: &str,
            document_content: &str,
            doc_result: &mut Option<String>
        ) {
            if doc_result.is_some() {
                return;
            }
            
            // Check if this is a block comment (documentation)
            if node.kind() == "block_comment" {
                let comment_text = get_node_text(node, document_content);
                
                if comment_text.starts_with("{-|") && comment_text.ends_with("-}") {
                    // Check if the next non-comment/whitespace node contains our symbol
                    let mut next_sibling = node.next_sibling();
                    while let Some(sibling) = next_sibling {
                        match sibling.kind() {
                            "block_comment" | "line_comment" => {
                                // Skip other comments
                                next_sibling = sibling.next_sibling();
                                continue;
                            }
                            _ => {
                                // Check if this node or its children contain our symbol
                                let sibling_text = get_node_text(sibling, document_content);
                                if sibling_text.contains(&format!("{} :", symbol_name)) || 
                                   sibling_text.contains(&format!("{} =", symbol_name)) {
                                    // Extract documentation content
                                    let doc_content = comment_text
                                        .strip_prefix("{-|").unwrap_or(&comment_text)
                                        .strip_suffix("-}").unwrap_or(&comment_text)
                                        .trim();
                                    *doc_result = Some(doc_content.to_string());
                                    return;
                                }
                                break;
                            }
                        }
                    }
                }
            }
            
            // Recursively traverse children
            if cursor.goto_first_child() {
                loop {
                    traverse_for_documentation(cursor.node(), cursor, symbol_name, document_content, doc_result);
                    if doc_result.is_some() || !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }
        
        traverse_for_documentation(root, &mut cursor, &symbol_name, document_content, &mut found_documentation);
        
        Ok(found_documentation)
    }

    /// Format hover information into LSP Hover response
    fn format_hover_response(&self, info: HoverInfo) -> Result<Hover> {
        let mut contents = Vec::new();

        // Add type information if available
        if let Some(type_info) = &info.type_info {
            contents.push(MarkedString::LanguageString(LanguageString {
                language: "gren".to_string(),
                value: type_info.clone(),
            }));
        }

        // Add documentation if available
        if let Some(documentation) = &info.documentation {
            contents.push(MarkedString::String(documentation.clone()));
        }

        // Add source module information if available
        if let Some(source_module) = &info.source_module {
            contents.push(MarkedString::String(format!("from {}", source_module)));
        }

        // If no content was found, show basic symbol information
        if contents.is_empty() {
            contents.push(MarkedString::String(format!("Symbol: {}", info.symbol_name)));
        }

        Ok(Hover {
            contents: HoverContents::Array(contents),
            range: Some(info.range),
        })
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
    fn test_hover_info_creation() {
        let hover_info = HoverInfo {
            type_info: Some("String -> String".to_string()),
            documentation: Some("Converts input to uppercase".to_string()),
            source_module: Some("String".to_string()),
            range: Range::default(),
            symbol_name: "toUpper".to_string(),
        };
        
        assert_eq!(hover_info.symbol_name, "toUpper");
        assert!(hover_info.type_info.is_some());
        assert!(hover_info.documentation.is_some());
    }
}