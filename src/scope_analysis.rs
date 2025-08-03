use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Query, QueryCursor, Node, Parser};
use tracing::debug;

use crate::gren_language;

/// Analyzes local scope for variables, parameters, and bindings
pub struct ScopeAnalysis {
    /// Query for extracting local variables and bindings
    local_vars_query: Query,
    /// Query for extracting function parameters
    params_query: Query,
    /// Query for extracting let bindings
    let_bindings_query: Query,
    /// Tree-sitter parser for Gren
    parser: Parser,
    /// Tree-sitter language for Gren
    language: tree_sitter::Language,
}

/// Information about the scope at a specific position
#[derive(Debug, Clone)]
pub struct ScopeInfo {
    /// Local variables available at this position
    pub local_variables: Vec<LocalVariable>,
    /// Function parameters in scope
    pub parameters: Vec<LocalVariable>,
    /// Let bindings in scope
    pub let_bindings: Vec<LocalVariable>,
    /// Nested scope depth
    pub scope_depth: usize,
}

/// Information about a local variable or binding
#[derive(Debug, Clone)]
pub struct LocalVariable {
    /// Variable name
    pub name: String,
    /// Type hint if available
    pub type_hint: Option<String>,
    /// Range where variable is defined
    pub definition_range: Range,
    /// Scope depth where variable is defined
    pub scope_depth: usize,
    /// Kind of variable
    pub kind: LocalVariableKind,
}

/// Different kinds of local variables
#[derive(Debug, Clone, PartialEq)]
pub enum LocalVariableKind {
    /// Function parameter
    Parameter,
    /// Let binding
    LetBinding,
    /// Pattern binding (from case expressions)
    PatternBinding,
    /// Loop variable
    LoopVariable,
}

impl ScopeAnalysis {
    /// Create a new scope analysis engine
    pub fn new() -> Result<Self> {
        let language = gren_language::language()?;
        let mut parser = Parser::new();
        parser.set_language(&language)
            .map_err(|_| anyhow!("Failed to set Gren language for parser"))?;

        // Query for function parameters
        let params_query_str = r#"
        ; Function parameters
        (function_declaration_left
          (lower_case_identifier) @param.name) @param.declaration
        "#;

        // Query for let bindings
        let let_bindings_query_str = r#"
        ; Let bindings
        (let_in_expr
          (value_declaration
            functionDeclarationLeft: (function_declaration_left
              (lower_case_identifier) @let.name) @let.params
            body: (_) @let.body) @let.declaration)
        "#;

        // Query for pattern bindings (from when expressions, destructuring)
        let local_vars_query_str = r#"
        ; Pattern variables in when expressions
        (when_is_expr
          branch: (when_is_branch
            pattern: (pattern) @pattern.root
            expr: (_) @pattern.expr))

        ; Pattern variables in function patterns
        (function_declaration_left
          (pattern) @pattern.param)*
        "#;

        let params_query = Query::new(&language, params_query_str)
            .map_err(|e| anyhow!("Failed to compile parameters query: {}", e))?;

        let let_bindings_query = Query::new(&language, let_bindings_query_str)
            .map_err(|e| anyhow!("Failed to compile let bindings query: {}", e))?;

        let local_vars_query = Query::new(&language, local_vars_query_str)
            .map_err(|e| anyhow!("Failed to compile local variables query: {}", e))?;

        Ok(Self {
            local_vars_query,
            params_query,
            let_bindings_query,
            parser,
            language,
        })
    }

    /// Analyze scope at a specific position in the document
    pub async fn analyze_scope_at_position(
        &mut self,
        content: &str,
        position: Position,
    ) -> Result<ScopeInfo> {
        // Parse the document
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse document for scope analysis"))?;

        let root_node = tree.root_node();
        
        // Convert LSP position to byte offset
        let byte_offset = position_to_byte_offset(content, position)?;
        
        // Find the node at the cursor position
        let cursor_node = root_node.descendant_for_byte_range(byte_offset, byte_offset)
            .unwrap_or(root_node);

        debug!("Analyzing scope at position {:?}, byte offset {}, node: {}", 
               position, byte_offset, cursor_node.kind());

        let mut scope_info = ScopeInfo {
            local_variables: Vec::new(),
            parameters: Vec::new(),
            let_bindings: Vec::new(),
            scope_depth: 0,
        };

        // Walk up the tree to find all scopes that contain the cursor
        let mut current_node = cursor_node;
        let mut depth = 0;

        loop {
            // Extract variables from current scope
            self.extract_scope_variables(&mut scope_info, current_node, content, depth)?;
            
            // Move to parent scope
            if let Some(parent) = current_node.parent() {
                current_node = parent;
                depth += 1;
            } else {
                break;
            }
        }

        scope_info.scope_depth = depth;
        
        // Sort variables by scope depth (closest scope first)
        scope_info.local_variables.sort_by_key(|var| var.scope_depth);
        scope_info.parameters.sort_by_key(|var| var.scope_depth);
        scope_info.let_bindings.sort_by_key(|var| var.scope_depth);

        debug!("Found {} local variables, {} parameters, {} let bindings at depth {}",
               scope_info.local_variables.len(),
               scope_info.parameters.len(), 
               scope_info.let_bindings.len(),
               scope_info.scope_depth);

        Ok(scope_info)
    }

    /// Extract variables from a specific scope node
    fn extract_scope_variables(
        &self,
        scope_info: &mut ScopeInfo,
        scope_node: Node,
        content: &str,
        depth: usize,
    ) -> Result<()> {
        // Extract function parameters if this is a function
        if self.is_function_scope(scope_node) {
            self.extract_function_parameters(scope_info, scope_node, content, depth)?;
        }

        // Extract let bindings if this is a let expression
        if self.is_let_scope(scope_node) {
            self.extract_let_bindings(scope_info, scope_node, content, depth)?;
        }

        // Extract pattern bindings if this is a when expression
        if self.is_when_scope(scope_node) {
            self.extract_pattern_bindings(scope_info, scope_node, content, depth)?;
        }

        Ok(())
    }

    /// Check if node represents a function scope
    fn is_function_scope(&self, node: Node) -> bool {
        matches!(node.kind(), "value_declaration" | "function_declaration_left")
    }

    /// Check if node represents a let scope
    fn is_let_scope(&self, node: Node) -> bool {
        node.kind() == "let_in_expr"
    }

    /// Check if node represents a when scope
    fn is_when_scope(&self, node: Node) -> bool {
        node.kind() == "when_is_branch"
    }

    /// Extract function parameters from a function node
    fn extract_function_parameters(
        &self,
        scope_info: &mut ScopeInfo,
        function_node: Node,
        content: &str,
        depth: usize,
    ) -> Result<()> {
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.params_query, function_node, content.as_bytes());

        for m in matches {
            for capture in m.captures {
                let capture_name = self.params_query.capture_names()[capture.index as usize];
                
                if capture_name == "param.name" {
                    let name = get_node_text(capture.node, content);
                    let range = node_to_range(capture.node);
                    
                    let variable = LocalVariable {
                        name,
                        type_hint: None, // Could be enhanced to extract type annotations
                        definition_range: range,
                        scope_depth: depth,
                        kind: LocalVariableKind::Parameter,
                    };
                    
                    scope_info.parameters.push(variable);
                }
            }
        }

        Ok(())
    }

    /// Extract let bindings from a let expression
    fn extract_let_bindings(
        &self,
        scope_info: &mut ScopeInfo,
        let_node: Node,
        content: &str,
        depth: usize,
    ) -> Result<()> {
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.let_bindings_query, let_node, content.as_bytes());

        for m in matches {
            for capture in m.captures {
                let capture_name = self.let_bindings_query.capture_names()[capture.index as usize];
                
                if capture_name == "let.name" {
                    let name = get_node_text(capture.node, content);
                    let range = node_to_range(capture.node);
                    
                    let variable = LocalVariable {
                        name,
                        type_hint: None,
                        definition_range: range,
                        scope_depth: depth,
                        kind: LocalVariableKind::LetBinding,
                    };
                    
                    scope_info.let_bindings.push(variable);
                }
            }
        }

        Ok(())
    }

    /// Extract pattern bindings from when expressions
    fn extract_pattern_bindings(
        &self,
        scope_info: &mut ScopeInfo,
        when_node: Node,
        content: &str,
        depth: usize,
    ) -> Result<()> {
        // Find pattern nodes and extract variable bindings
        let mut cursor = when_node.walk();
        
        for child in when_node.children(&mut cursor) {
            if child.kind() == "pattern" {
                self.extract_pattern_variables(scope_info, child, content, depth)?;
            }
        }

        Ok(())
    }

    /// Extract variable names from a pattern node
    fn extract_pattern_variables(
        &self,
        scope_info: &mut ScopeInfo,
        pattern_node: Node,
        content: &str,
        depth: usize,
    ) -> Result<()> {
        let mut cursor = pattern_node.walk();
        
        // Recursively search for identifier nodes in patterns
        self.visit_pattern_identifiers(scope_info, pattern_node, content, depth, &mut cursor);
        
        Ok(())
    }

    /// Recursively visit pattern nodes to find identifiers
    fn visit_pattern_identifiers<'a>(
        &self,
        scope_info: &mut ScopeInfo,
        node: Node<'a>,
        content: &str,
        depth: usize,
        _cursor: &mut tree_sitter::TreeCursor<'a>,
    ) {
        if node.kind() == "lower_case_identifier" {
            let name = get_node_text(node, content);
            let range = node_to_range(node);
            
            let variable = LocalVariable {
                name,
                type_hint: None,
                definition_range: range,
                scope_depth: depth,
                kind: LocalVariableKind::PatternBinding,
            };
            
            scope_info.local_variables.push(variable);
        }

        // Visit children
        let mut child_cursor = node.walk();
        let children: Vec<_> = node.children(&mut child_cursor).collect();
        for child in children {
            self.visit_pattern_identifiers(scope_info, child, content, depth, &mut child.walk());
        }
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
    fn test_scope_analysis_creation() {
        let result = ScopeAnalysis::new();
        match result {
            Ok(_) => {
                // Success
            }
            Err(e) => {
                panic!("ScopeAnalysis should create successfully, but got error: {}", e);
            }
        }
    }

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
    fn test_local_variable_creation() {
        let var = LocalVariable {
            name: "test".to_string(),
            type_hint: Some("String".to_string()),
            definition_range: Range::default(),
            scope_depth: 1,
            kind: LocalVariableKind::Parameter,
        };
        
        assert_eq!(var.name, "test");
        assert_eq!(var.kind, LocalVariableKind::Parameter);
        assert_eq!(var.scope_depth, 1);
    }
}