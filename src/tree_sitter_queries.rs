use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Query, QueryCursor, Tree, Node};
use tracing::debug;
use url::Url;

use crate::symbol_index::Symbol;

/// Tree-sitter query engine for extracting symbols from Gren AST
#[derive(Debug)]
pub struct GrenQueryEngine {
    /// Query for extracting function definitions
    function_query: Query,
    /// Query for extracting type definitions  
    type_query: Query,
    /// Query for extracting import statements
    import_query: Query,
    /// Query for extracting constants/variables
    constant_query: Query,
    /// Query for extracting module declarations
    module_query: Query,
    /// Tree-sitter language for Gren
    language: tree_sitter::Language,
}

impl GrenQueryEngine {
    /// Create a new query engine with all Gren symbol extraction queries
    pub fn new() -> Result<Self> {
        // Get the Gren language (assuming it's loaded externally)
        let language = get_gren_language()?;

        // Function definitions query - extracts function declarations with type annotations
        let function_query_str = r#"
        ; Function type annotation
        (type_annotation
          name: (lower_case_identifier) @function.name
          typeExpression: (type_expression) @function.type) @function.annotation

        ; Function value declaration  
        (value_declaration
          functionDeclarationLeft: (function_declaration_left
            name: (lower_case_identifier) @function.name
            patterns: (pattern)*) @function.params
          expression: (_) @function.body) @function.declaration
        "#;

        // Type definitions query - extracts custom types and type aliases
        let type_query_str = r#"
        ; Custom type declarations
        (type_declaration
          name: (upper_case_identifier) @type.name
          unionVariant: (union_variant
            name: (upper_case_identifier) @variant.name)*) @type.declaration

        ; Type alias declarations
        (type_alias_declaration
          name: (upper_case_identifier) @alias.name
          typeExpression: (type_expression) @alias.type) @alias.declaration
        "#;

        // Import statements query - for cross-module resolution
        let import_query_str = r#"
        ; Import clauses
        (import_clause
          moduleName: (upper_case_qid) @import.module
          asClause: (as_clause
            name: (upper_case_identifier) @import.alias)?
          exposing: (exposing_list
            (exposed_value (lower_case_identifier) @import.value)*
            (exposed_type (upper_case_identifier) @import.type)*
            doubleDot: (double_dot) @import.all)?) @import.declaration
        "#;

        // Constants and module-level values query
        let constant_query_str = r#"
        ; Top-level constants (value declarations without parameters)
        (value_declaration
          functionDeclarationLeft: (function_declaration_left
            name: (lower_case_identifier) @constant.name
            ; No patterns = constant
            ) @constant.left
          expression: (_) @constant.value) @constant.declaration
        "#;

        // Module declaration query
        let module_query_str = r#"
        ; Module declarations
        (module_declaration
          name: (upper_case_qid) @module.name
          exposing: (exposing_list) @module.exposing) @module.declaration
        "#;

        let function_query = Query::new(&language, function_query_str)
            .map_err(|e| anyhow!("Failed to compile function query: {}", e))?;

        let type_query = Query::new(&language, type_query_str)
            .map_err(|e| anyhow!("Failed to compile type query: {}", e))?;

        let import_query = Query::new(&language, import_query_str)
            .map_err(|e| anyhow!("Failed to compile import query: {}", e))?;

        let constant_query = Query::new(&language, constant_query_str)
            .map_err(|e| anyhow!("Failed to compile constant query: {}", e))?;

        let module_query = Query::new(&language, module_query_str)
            .map_err(|e| anyhow!("Failed to compile module query: {}", e))?;

        Ok(Self {
            function_query,
            type_query,
            import_query,
            constant_query,
            module_query,
            language,
        })
    }

    /// Extract all symbols from a Gren file
    pub fn extract_symbols(&self, uri: &Url, tree: &Tree, source: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let root_node = tree.root_node();

        // Extract functions
        symbols.extend(self.extract_functions(uri, &root_node, source)?);

        // Extract types
        symbols.extend(self.extract_types(uri, &root_node, source)?);

        // Extract constants
        symbols.extend(self.extract_constants(uri, &root_node, source)?);

        // Extract module declaration
        symbols.extend(self.extract_module(uri, &root_node, source)?);

        debug!("Extracted {} symbols from {}", symbols.len(), uri);
        Ok(symbols)
    }

    /// Extract function definitions
    fn extract_functions(&self, uri: &Url, node: &Node, source: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.function_query, *node, source.as_bytes());

        let mut function_types = std::collections::HashMap::new();

        // First pass: collect type annotations
        for m in matches {
            for capture in m.captures {
                let capture_name = self.function_query.capture_names()[capture.index as usize];
                
                if capture_name == "function.name" && m.pattern_index == 0 {
                    // This is from a type annotation
                    let name = get_node_text(capture.node, source);
                    if let Some(type_match) = m.captures.iter().find(|c| 
                        self.function_query.capture_names()[c.index as usize] == "function.type") {
                        let type_text = get_node_text(type_match.node, source);
                        function_types.insert(name.clone(), type_text);
                    }
                }
            }
        }

        // Second pass: extract function declarations
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.function_query, *node, source.as_bytes());

        for m in matches {
            if m.pattern_index == 1 { // Function declaration pattern
                let mut name = String::new();
                let mut range = Range::default();

                for capture in m.captures {
                    let capture_name = self.function_query.capture_names()[capture.index as usize];
                    
                    match capture_name {
                        "function.name" => {
                            name = get_node_text(capture.node, source);
                            range = node_to_range(capture.node);
                        }
                        "function.declaration" => {
                            range = node_to_range(capture.node);
                        }
                        _ => {}
                    }
                }

                if !name.is_empty() {
                    let signature = function_types.get(&name).cloned()
                        .map(|type_text| format!("{} : {}", name, type_text));

                    let symbol = Symbol::new(
                        name,
                        SymbolKind::FUNCTION,
                        uri,
                        range,
                        None, // container
                        signature,
                        None, // documentation
                    );
                    symbols.push(symbol);
                }
            }
        }

        Ok(symbols)
    }

    /// Extract type definitions (custom types and type aliases)
    fn extract_types(&self, uri: &Url, node: &Node, source: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.type_query, *node, source.as_bytes());

        for m in matches {
            match m.pattern_index {
                0 => {
                    // Custom type declaration
                    let mut type_name = String::new();
                    let mut type_range = Range::default();
                    let mut variants = Vec::new();

                    for capture in m.captures {
                        let capture_name = self.type_query.capture_names()[capture.index as usize];
                        
                        match capture_name {
                            "type.name" => {
                                type_name = get_node_text(capture.node, source);
                            }
                            "type.declaration" => {
                                type_range = node_to_range(capture.node);
                            }
                            "variant.name" => {
                                variants.push(get_node_text(capture.node, source));
                            }
                            _ => {}
                        }
                    }

                    if !type_name.is_empty() {
                        let signature = if variants.is_empty() {
                            format!("type {}", type_name)
                        } else {
                            format!("type {} = {}", type_name, variants.join(" | "))
                        };

                        let symbol = Symbol::new(
                            type_name,
                            SymbolKind::ENUM, // Custom types are like enums
                            uri,
                            type_range,
                            None,
                            Some(signature),
                            None,
                        );
                        symbols.push(symbol);
                    }
                }
                1 => {
                    // Type alias declaration
                    let mut alias_name = String::new();
                    let mut alias_range = Range::default();

                    for capture in m.captures {
                        let capture_name = self.type_query.capture_names()[capture.index as usize];
                        
                        match capture_name {
                            "alias.name" => {
                                alias_name = get_node_text(capture.node, source);
                            }
                            "alias.declaration" => {
                                alias_range = node_to_range(capture.node);
                            }
                            _ => {}
                        }
                    }

                    if !alias_name.is_empty() {
                        let symbol = Symbol::new(
                            alias_name.clone(),
                            SymbolKind::STRUCT, // Type aliases are like structs
                            uri,
                            alias_range,
                            None,
                            Some(format!("type alias {}", alias_name)),
                            None,
                        );
                        symbols.push(symbol);
                    }
                }
                _ => {}
            }
        }

        Ok(symbols)
    }

    /// Extract constants (module-level values without parameters)
    fn extract_constants(&self, uri: &Url, node: &Node, source: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.constant_query, *node, source.as_bytes());

        for m in matches {
            let mut name = String::new();
            let mut range = Range::default();

            for capture in m.captures {
                let capture_name = self.constant_query.capture_names()[capture.index as usize];
                
                match capture_name {
                    "constant.name" => {
                        name = get_node_text(capture.node, source);
                    }
                    "constant.declaration" => {
                        range = node_to_range(capture.node);
                    }
                    _ => {}
                }
            }

            if !name.is_empty() {
                let symbol = Symbol::new(
                    name,
                    SymbolKind::CONSTANT,
                    uri,
                    range,
                    None,
                    None,
                    None,
                );
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Extract module declaration
    fn extract_module(&self, uri: &Url, node: &Node, source: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.module_query, *node, source.as_bytes());

        for m in matches {
            let mut name = String::new();
            let mut range = Range::default();

            for capture in m.captures {
                let capture_name = self.module_query.capture_names()[capture.index as usize];
                
                match capture_name {
                    "module.name" => {
                        name = get_node_text(capture.node, source);
                    }
                    "module.declaration" => {
                        range = node_to_range(capture.node);
                    }
                    _ => {}
                }
            }

            if !name.is_empty() {
                let symbol = Symbol::new(
                    name,
                    SymbolKind::MODULE,
                    uri,
                    range,
                    None,
                    None,
                    None,
                );
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Extract import information for cross-module resolution
    pub fn extract_imports(&self, uri: &Url, tree: &Tree, source: &str) -> Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        let root_node = tree.root_node();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.import_query, root_node, source.as_bytes());

        for m in matches {
            let mut module_name = String::new();
            let mut alias_name = None;
            let mut imported_symbols = Vec::new();
            let mut exposing_all = false;

            for capture in m.captures {
                let capture_name = self.import_query.capture_names()[capture.index as usize];
                
                match capture_name {
                    "import.module" => {
                        module_name = get_node_text(capture.node, source);
                    }
                    "import.alias" => {
                        alias_name = Some(get_node_text(capture.node, source));
                    }
                    "import.value" => {
                        imported_symbols.push(get_node_text(capture.node, source));
                    }
                    "import.type" => {
                        imported_symbols.push(get_node_text(capture.node, source));
                    }
                    "import.all" => {
                        exposing_all = true;
                    }
                    _ => {}
                }
            }

            if !module_name.is_empty() {
                let import_info = ImportInfo::new(
                    uri,
                    module_name,
                    if imported_symbols.is_empty() { None } else { Some(imported_symbols) },
                    alias_name,
                    exposing_all,
                );
                imports.push(import_info);
            }
        }

        debug!("Extracted {} imports from {}", imports.len(), uri);
        Ok(imports)
    }
}

/// Import information for cross-module resolution  
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub source_uri: String,
    pub imported_module: String,
    pub imported_symbols: Option<Vec<String>>,
    pub alias_name: Option<String>,
    pub exposing_all: bool,
}

impl ImportInfo {
    pub fn new(
        source_uri: &Url,
        imported_module: String,
        imported_symbols: Option<Vec<String>>,
        alias_name: Option<String>,
        exposing_all: bool,
    ) -> Self {
        Self {
            source_uri: source_uri.to_string(),
            imported_module,
            imported_symbols, 
            alias_name,
            exposing_all,
        }
    }
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

/// Get the Gren tree-sitter language
/// This would normally load the compiled Gren grammar
/// For now, this is a placeholder that will need to be implemented
/// when the actual Gren tree-sitter grammar is available
fn get_gren_language() -> Result<tree_sitter::Language> {
    // TODO: Load the actual Gren tree-sitter grammar
    // This would typically be:
    // extern "C" { fn tree_sitter_gren() -> tree_sitter::Language; }
    // unsafe { tree_sitter_gren() }
    
    // For now, return an error indicating this needs to be implemented
    Err(anyhow!("Gren tree-sitter grammar not yet integrated. This will be implemented when the tree-sitter-gren grammar is available."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_compilation() {
        // Test that all queries compile successfully
        // This will fail until we have the actual Gren grammar, but it's good to have
        let result = GrenQueryEngine::new();
        
        // For now, we expect this to fail with our placeholder implementation
        match result {
            Err(e) => assert!(e.to_string().contains("Gren tree-sitter grammar not yet integrated")),
            Ok(_) => panic!("Expected error due to missing Gren grammar"),
        }
    }

    #[test]
    fn test_node_to_range() {
        // Test the range conversion utility
        // This would need a mock node for proper testing
        // For now, just ensure the function exists and compiles
    }

    #[test]
    fn test_import_info_creation() {
        let uri = Url::parse("file:///test.gren").unwrap();
        let import = ImportInfo::new(
            &uri,
            "Http.Request".to_string(),
            Some(vec!["get".to_string(), "post".to_string()]),
            Some("Request".to_string()),
            false,
        );

        assert_eq!(import.source_uri, uri.to_string());
        assert_eq!(import.imported_module, "Http.Request");
        assert_eq!(import.imported_symbols, Some(vec!["get".to_string(), "post".to_string()]));
        assert_eq!(import.alias_name, Some("Request".to_string()));
        assert!(!import.exposing_all);
    }
}