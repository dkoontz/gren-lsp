use crate::symbol_index::SymbolIndex;
use crate::gren_language;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tracing::{debug, error, info};
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

/// Errors that can occur during code action generation
#[derive(Debug, thiserror::Error)]
pub enum CodeActionError {
    #[error("Tree-sitter parsing failed: {0}")]
    ParsingError(String),
    
    #[error("Query execution failed: {0}")]
    QueryError(String),
    
    #[error("Symbol index error: {0}")]
    SymbolIndexError(String),
    
    #[error("Invalid range: {0}")]
    InvalidRange(String),
    
    #[error("Document processing error: {0}")]
    DocumentError(String),
}

pub type CodeActionResult<T> = Result<T, CodeActionError>;

/// Engine for generating code actions based on LSP requests
pub struct CodeActionsEngine {
    /// Symbol index for resolving imports and symbols
    symbol_index: Arc<RwLock<Option<SymbolIndex>>>,
    /// Tree-sitter parser for Gren language
    parser: Parser,
    /// Tree-sitter language for Gren
    language: Language,
}

impl CodeActionsEngine {
    /// Create a new code actions engine
    pub fn new(symbol_index: Arc<RwLock<Option<SymbolIndex>>>) -> CodeActionResult<Self> {
        let language = gren_language::language()
            .map_err(|e| CodeActionError::ParsingError(format!("Failed to get Gren language: {}", e)))?;
        
        let mut parser = Parser::new();
        parser.set_language(&language)
            .map_err(|e| CodeActionError::ParsingError(format!("Failed to set parser language: {}", e)))?;
        
        Ok(Self {
            symbol_index,
            parser,
            language,
        })
    }
    
    /// Handle textDocument/codeAction LSP request
    pub async fn handle_code_action(
        &mut self,
        params: CodeActionParams,
        document_content: &str,
    ) -> CodeActionResult<Option<Vec<CodeActionOrCommand>>> {
        let uri = &params.text_document.uri;
        let range = &params.range;
        let context = &params.context;
        
        info!("🔧 Code action request for {} at range {:?}", uri, range);
        
        // Parse the document
        let tree = self.parser.parse(document_content, None)
            .ok_or_else(|| CodeActionError::ParsingError("Failed to parse document".to_string()))?;
        
        let mut actions = Vec::new();
        
        // Generate diagnostic-based actions
        if !context.diagnostics.is_empty() {
            debug!("Generating actions for {} diagnostics", context.diagnostics.len());
            let diagnostic_actions = self.generate_diagnostic_actions(
                uri,
                document_content,
                &tree,
                &context.diagnostics,
            ).await?;
            actions.extend(diagnostic_actions);
        }
        
        // Generate cursor-based actions only if no diagnostics were provided
        // This avoids overwhelming users with unrelated suggestions when there are compile errors
        if context.diagnostics.is_empty() {
            let cursor_actions = self.generate_cursor_based_actions(
                uri,
                document_content,
                &tree,
                range,
            ).await?;
            actions.extend(cursor_actions);
        }
        
        // Filter actions based on requested kinds
        if let Some(only_kinds) = &context.only {
            actions.retain(|action| {
                if let CodeActionOrCommand::CodeAction(code_action) = action {
                    if let Some(ref kind) = code_action.kind {
                        only_kinds.iter().any(|requested_kind| {
                            kind.as_str().starts_with(requested_kind.as_str())
                        })
                    } else {
                        false
                    }
                } else {
                    true // Always include commands
                }
            });
        }
        
        info!("🔧 Generated {} code actions", actions.len());
        
        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
    
    /// Generate code actions based on compiler diagnostics
    async fn generate_diagnostic_actions(
        &mut self,
        uri: &Url,
        document_content: &str,
        tree: &Tree,
        diagnostics: &[Diagnostic],
    ) -> CodeActionResult<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();
        
        for diagnostic in diagnostics {
            // Check for missing import errors
            if self.is_missing_import_error(&diagnostic.message) {
                if let Some(action) = self.generate_missing_import_action(
                    uri,
                    document_content,
                    tree,
                    diagnostic,
                ).await? {
                    actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }
            
            // Check for type mismatch errors
            if self.is_type_mismatch_error(&diagnostic.message) {
                if let Some(action) = self.generate_type_mismatch_action(
                    uri,
                    document_content,
                    tree,
                    diagnostic,
                ).await? {
                    actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }
            
            // Check for unused import warnings
            if self.is_unused_import_warning(&diagnostic.message) {
                if let Some(action) = self.generate_remove_unused_import_action(
                    uri,
                    document_content,
                    tree,
                    diagnostic,
                ).await? {
                    actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }
        }
        
        Ok(actions)
    }
    
    /// Generate cursor-based code actions (not tied to specific diagnostics)
    async fn generate_cursor_based_actions(
        &mut self,
        uri: &Url,
        document_content: &str,
        tree: &Tree,
        range: &Range,
    ) -> CodeActionResult<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();
        
        // Check if cursor is on a function without type signature
        if let Some(action) = self.generate_add_type_signature_action(
            uri,
            document_content,
            tree,
            range,
        ).await? {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
        
        Ok(actions)
    }
    
    /// Check if diagnostic message indicates a missing import error
    fn is_missing_import_error(&self, message: &str) -> bool {
        message.contains("I cannot find") ||
        message.contains("not in scope") ||
        message.contains("undefined") ||
        message.contains("not found")
    }
    
    /// Check if diagnostic message indicates a type mismatch error
    fn is_type_mismatch_error(&self, message: &str) -> bool {
        message.contains("type mismatch") ||
        message.contains("expected") && message.contains("but got") ||
        message.contains("cannot match")
    }
    
    /// Check if diagnostic message indicates an unused import warning
    fn is_unused_import_warning(&self, message: &str) -> bool {
        message.contains("unused import") ||
        message.contains("imported but not used")
    }
    
    /// Generate a code action to add a missing import
    async fn generate_missing_import_action(
        &mut self,
        uri: &Url,
        document_content: &str,
        tree: &Tree,
        diagnostic: &Diagnostic,
    ) -> CodeActionResult<Option<CodeAction>> {
        // Extract the undefined symbol from the diagnostic message
        let symbol_name = self.extract_undefined_symbol(&diagnostic.message);
        if symbol_name.is_none() {
            debug!("Could not extract symbol name from diagnostic: {}", diagnostic.message);
            return Ok(None);
        }
        let symbol_name = symbol_name.unwrap();
        
        // Find the position to insert the import (after existing imports) first
        let import_position = self.find_import_insertion_position(document_content, tree)?;
        
        // Query symbol index to find possible imports
        let matching_symbols = {
            let symbol_index = self.symbol_index.read().await;
            let symbol_index = match symbol_index.as_ref() {
                Some(index) => index,
                None => {
                    debug!("Symbol index not available for import suggestions");
                    return Ok(None);
                }
            };
            
            // Find symbols that match the undefined symbol
            symbol_index.find_symbols_by_name(&symbol_name).await
                .map_err(|e| CodeActionError::SymbolIndexError(e.to_string()))?
        };
        
        if matching_symbols.is_empty() {
            debug!("No matching symbols found for '{}'", symbol_name);
            return Ok(None);
        }
        
        // Use the first matching symbol (in a real implementation, we might want to be smarter about this)
        let symbol = &matching_symbols[0];
        let default_module = "Unknown".to_string();
        let module_name = symbol.container.as_ref().unwrap_or(&default_module);
        
        // Generate the import statement
        let import_text = format!("import {} exposing ({})", module_name, symbol_name);
        
        // Create the workspace edit
        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                changes.insert(uri.clone(), vec![TextEdit {
                    range: Range {
                        start: import_position,
                        end: import_position,
                    },
                    new_text: format!("{}\n", import_text),
                }]);
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };
        
        let code_action = CodeAction {
            title: format!("Import {} from {}", symbol_name, module_name),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            is_preferred: Some(true),
            disabled: None,
            edit: Some(edit),
            command: None,
            data: None,
        };
        
        Ok(Some(code_action))
    }
    
    /// Generate a code action to fix type mismatches
    async fn generate_type_mismatch_action(
        &mut self,
        uri: &Url,
        _document_content: &str,
        _tree: &Tree,
        diagnostic: &Diagnostic,
    ) -> CodeActionResult<Option<CodeAction>> {
        // Extract type information from the diagnostic message
        let type_fix = self.extract_type_mismatch_fix(&diagnostic.message);
        if type_fix.is_none() {
            debug!("Could not extract type fix information from diagnostic: {}", diagnostic.message);
            return Ok(None);
        }
        let (expected_type, actual_type, suggested_fix) = type_fix.unwrap();
        
        // Create a workspace edit to apply the type fix
        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = std::collections::HashMap::new();
                changes.insert(uri.clone(), vec![TextEdit {
                    range: diagnostic.range,
                    new_text: suggested_fix.clone(),
                }]);
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };
        
        let code_action = CodeAction {
            title: format!("Convert {} to {}", actual_type, expected_type),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            is_preferred: Some(false),
            disabled: None,
            edit: Some(edit),
            command: None,
            data: None,
        };
        
        Ok(Some(code_action))
    }
    
    /// Generate a code action to remove unused imports
    async fn generate_remove_unused_import_action(
        &mut self,
        uri: &Url,
        document_content: &str,
        tree: &Tree,
        diagnostic: &Diagnostic,
    ) -> CodeActionResult<Option<CodeAction>> {
        // Find the import statement to remove based on the diagnostic range
        let import_range = self.find_import_at_range(document_content, tree, &diagnostic.range)?;
        if import_range.is_none() {
            debug!("Could not find import statement at diagnostic range");
            return Ok(None);
        }
        let import_range = import_range.unwrap();
        
        // Create the workspace edit to remove the import
        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                changes.insert(uri.clone(), vec![TextEdit {
                    range: import_range,
                    new_text: String::new(), // Remove the import
                }]);
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };
        
        let code_action = CodeAction {
            title: "Remove unused import".to_string(),
            kind: Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
            diagnostics: Some(vec![diagnostic.clone()]),
            is_preferred: Some(true),
            disabled: None,
            edit: Some(edit),
            command: None,
            data: None,
        };
        
        Ok(Some(code_action))
    }
    
    /// Generate a code action to add type signature to a function
    async fn generate_add_type_signature_action(
        &mut self,
        uri: &Url,
        document_content: &str,
        tree: &Tree,
        range: &Range,
    ) -> CodeActionResult<Option<CodeAction>> {
        // Find function definition at the cursor position
        let function_info = self.find_function_at_position(document_content, tree, range)?;
        if function_info.is_none() {
            return Ok(None);
        }
        let (function_name, function_range, has_type_sig) = function_info.unwrap();
        
        // Skip if function already has a type signature
        if has_type_sig {
            return Ok(None);
        }
        
        // Generate a placeholder type signature (in a real implementation, we'd infer the actual types)
        let type_signature = format!("{} : ()", function_name);
        
        // Create the workspace edit to add the type signature
        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                changes.insert(uri.clone(), vec![TextEdit {
                    range: Range {
                        start: function_range.start,
                        end: function_range.start,
                    },
                    new_text: format!("{}\n", type_signature),
                }]);
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };
        
        let code_action = CodeAction {
            title: format!("Add type signature for {}", function_name),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            diagnostics: None,
            is_preferred: Some(false),
            disabled: None,
            edit: Some(edit),
            command: None,
            data: None,
        };
        
        Ok(Some(code_action))
    }
    
    /// Extract undefined symbol name from compiler diagnostic message
    fn extract_undefined_symbol(&self, message: &str) -> Option<String> {
        // This is a simplified implementation - real implementation would parse 
        // actual Gren compiler error messages more precisely
        if let Some(start) = message.find("'") {
            if let Some(end) = message[start + 1..].find("'") {
                return Some(message[start + 1..start + 1 + end].to_string());
            }
        }
        
        // Try other patterns
        if message.contains("I cannot find") {
            // Extract symbol after "I cannot find "
            if let Some(start) = message.find("I cannot find ") {
                let start = start + "I cannot find ".len();
                if let Some(end) = message[start..].find(char::is_whitespace) {
                    return Some(message[start..start + end].to_string());
                }
            }
        }
        
        None
    }
    
    /// Extract type mismatch information and suggest a fix
    fn extract_type_mismatch_fix(&self, message: &str) -> Option<(String, String, String)> {
        // This is a simplified implementation for common type mismatch patterns in Gren
        // Pattern: "expected X but got Y" or "type mismatch: expected X, got Y"
        
        if message.contains("expected") && message.contains("but got") {
            // Try to extract "expected X but got Y"
            if let Some(expected_start) = message.find("expected ") {
                let expected_start = expected_start + "expected ".len();
                if let Some(expected_end) = message[expected_start..].find(" but got") {
                    let expected_type = message[expected_start..expected_start + expected_end].trim();
                    
                    if let Some(got_start) = message.find("but got ") {
                        let got_start = got_start + "but got ".len();
                        let actual_type = message[got_start..].trim_end_matches('.').trim();
                        
                        // Generate simple conversion suggestions
                        if let Some(fix) = self.suggest_type_conversion(expected_type, actual_type) {
                            return Some((expected_type.to_string(), actual_type.to_string(), fix));
                        }
                    }
                }
            }
        }
        
        // Pattern: "cannot match X with Y"
        if message.contains("cannot match") && message.contains("with") {
            if let Some(match_start) = message.find("cannot match ") {
                let match_start = match_start + "cannot match ".len();
                if let Some(with_pos) = message[match_start..].find(" with ") {
                    let actual_type = message[match_start..match_start + with_pos].trim();
                    let with_start = match_start + with_pos + " with ".len();
                    let expected_type = message[with_start..].trim_end_matches('.').trim();
                    
                    if let Some(fix) = self.suggest_type_conversion(expected_type, actual_type) {
                        return Some((expected_type.to_string(), actual_type.to_string(), fix));
                    }
                }
            }
        }
        
        None
    }
    
    /// Suggest type conversion based on expected and actual types
    fn suggest_type_conversion(&self, expected: &str, actual: &str) -> Option<String> {
        match (expected, actual) {
            // String to Int conversion
            ("Int", "String") => Some("String.toInt".to_string()),
            ("String", "Int") => Some("String.fromInt".to_string()),
            
            // Float conversions
            ("Float", "Int") => Some("toFloat".to_string()),
            ("Int", "Float") => Some("round".to_string()),
            
            // Maybe wrapping
            (expected, actual) if expected.starts_with("Maybe ") => {
                let inner_type = expected.strip_prefix("Maybe ").unwrap_or(expected);
                if actual == inner_type {
                    Some("Just".to_string())
                } else {
                    None
                }
            }
            
            // Array to List conversions (Gren uses arrays, but might have List types)
            ("Array", "List") => Some("Array.fromList".to_string()),
            ("List", "Array") => Some("Array.toList".to_string()),
            
            _ => None, // No simple conversion available
        }
    }
    
    /// Find the position where a new import should be inserted
    fn find_import_insertion_position(&mut self, document_content: &str, tree: &Tree) -> CodeActionResult<Position> {
        // Use tree-sitter to find the last import clause
        let import_query = Query::new(&self.language, "(import_clause) @import")
            .map_err(|e| CodeActionError::QueryError(format!("Failed to create import query: {}", e)))?;
        
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&import_query, tree.root_node(), document_content.as_bytes());
        
        let mut last_import_line = 0;
        for m in matches {
            for capture in m.captures {
                let end_position = capture.node.end_position();
                last_import_line = last_import_line.max(end_position.row);
            }
        }
        
        // Insert after the last import, or at the beginning if no imports
        Ok(Position {
            line: last_import_line as u32,
            character: 0,
        })
    }
    
    /// Find import clause at the given range
    fn find_import_at_range(&mut self, document_content: &str, tree: &Tree, range: &Range) -> CodeActionResult<Option<Range>> {
        let import_query = Query::new(&self.language, "(import_clause) @import")
            .map_err(|e| CodeActionError::QueryError(format!("Failed to create import query: {}", e)))?;
        
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&import_query, tree.root_node(), document_content.as_bytes());
        
        for m in matches {
            for capture in m.captures {
                let node_start = capture.node.start_position();
                let node_end = capture.node.end_position();
                
                let node_range = Range {
                    start: Position {
                        line: node_start.row as u32,
                        character: node_start.column as u32,
                    },
                    end: Position {
                        line: node_end.row as u32,
                        character: node_end.column as u32,
                    },
                };
                
                // Check if this import overlaps with the diagnostic range
                if ranges_overlap(&node_range, range) {
                    // Include the newline in the range to remove
                    return Ok(Some(Range {
                        start: node_range.start,
                        end: Position {
                            line: node_range.end.line + 1,
                            character: 0,
                        },
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Find function definition at the given position
    fn find_function_at_position(
        &mut self,
        document_content: &str,
        tree: &Tree,
        range: &Range,
    ) -> CodeActionResult<Option<(String, Range, bool)>> {
        // Query for function definitions (using value_declaration with function_declaration_left)
        let function_query = Query::new(
            &self.language,
            "(value_declaration functionDeclarationLeft: (function_declaration_left (lower_case_identifier) @name)) @function"
        ).map_err(|e| CodeActionError::QueryError(format!("Failed to create function query: {}", e)))?;
        
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&function_query, tree.root_node(), document_content.as_bytes());
        
        for m in matches {
            let mut function_node = None;
            let mut name_node = None;
            
            for capture in m.captures {
                match function_query.capture_names()[capture.index as usize] {
                    "function" => function_node = Some(capture.node),
                    "name" => name_node = Some(capture.node),
                    _ => {}
                }
            }
            
            if let (Some(func_node), Some(name_node)) = (function_node, name_node) {
                let func_start = func_node.start_position();
                let func_end = func_node.end_position();
                
                let func_range = Range {
                    start: Position {
                        line: func_start.row as u32,
                        character: func_start.column as u32,
                    },
                    end: Position {
                        line: func_end.row as u32,
                        character: func_end.column as u32,
                    },
                };
                
                // Check if the cursor is within this function
                if position_in_range(&range.start, &func_range) {
                    let function_name = name_node.utf8_text(document_content.as_bytes())
                        .map_err(|e| CodeActionError::DocumentError(format!("Failed to extract function name: {}", e)))?
                        .to_string();
                    
                    // Check if function already has a type signature
                    let has_type_sig = self.check_function_has_type_signature(document_content, tree, &function_name, &func_range)?;
                    
                    return Ok(Some((function_name, func_range, has_type_sig)));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Check if a function already has a type signature
    fn check_function_has_type_signature(
        &mut self,
        document_content: &str,
        tree: &Tree,
        function_name: &str,
        function_range: &Range,
    ) -> CodeActionResult<bool> {
        // Look for type annotations before the function
        let type_annotation_query = Query::new(
            &self.language,
            "(type_annotation name: (lower_case_identifier) @name) @annotation"
        ).map_err(|e| CodeActionError::QueryError(format!("Failed to create type annotation query: {}", e)))?;
        
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&type_annotation_query, tree.root_node(), document_content.as_bytes());
        
        for m in matches {
            for capture in m.captures {
                if let "name" = type_annotation_query.capture_names()[capture.index as usize] {
                    let name_text = capture.node.utf8_text(document_content.as_bytes())
                        .map_err(|e| CodeActionError::DocumentError(format!("Failed to extract annotation name: {}", e)))?;
                    
                    if name_text == function_name {
                        let annotation_end = capture.node.end_position();
                        let annotation_line = annotation_end.row as u32;
                        
                        // Check if the type annotation is immediately before the function
                        if annotation_line + 1 >= function_range.start.line {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
}

/// Check if two ranges overlap
fn ranges_overlap(range1: &Range, range2: &Range) -> bool {
    range1.start <= range2.end && range2.start <= range1.end
}

/// Check if a position is within a range
fn position_in_range(position: &Position, range: &Range) -> bool {
    (position.line > range.start.line || 
     (position.line == range.start.line && position.character >= range.start.character)) &&
    (position.line < range.end.line || 
     (position.line == range.end.line && position.character <= range.end.character))
}