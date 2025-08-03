use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Query, QueryCursor, Tree, Node};
use tracing::debug;
use url::Url;

use crate::symbol_index::{Symbol, SymbolReference, ReferenceKind};
use crate::gren_language;

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
    /// Query for extracting symbol references/usages
    reference_query: Query,
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
          (lower_case_identifier) @function.name
          (type_expression) @function.type) @function.annotation

        ; Function value declaration  
        (value_declaration
          (function_declaration_left
            (lower_case_identifier) @function.name) @function.params
          body: (_) @function.body) @function.declaration
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
        ; Import clauses - basic structure
        (import_clause
          moduleName: (upper_case_qid) @import.module) @import.declaration

        ; Import alias
        (import_clause
          asClause: (as_clause
            name: (upper_case_identifier) @import.alias))

        ; Exposed values within exposing list  
        (import_clause
          exposing: (exposing_list
            (exposed_value (lower_case_identifier) @import.value)))

        ; Exposed types within exposing list
        (import_clause
          exposing: (exposing_list  
            (exposed_type (upper_case_identifier) @import.type)))

        ; Double dot for exposing all
        (import_clause
          exposing: (exposing_list
            (double_dot) @import.all))
        "#;

        // Constants and module-level values query
        let constant_query_str = r#"
        ; Top-level constants (value declarations without parameters)
        (value_declaration
          functionDeclarationLeft: (function_declaration_left
            (lower_case_identifier) @constant.name
            ; No patterns = constant
            ) @constant.left
          body: (_) @constant.value) @constant.declaration
        "#;

        // Module declaration query
        let module_query_str = r#"
        ; Module declarations
        (module_declaration
          name: (upper_case_qid) @module.name
          exposing: (exposing_list) @module.exposing) @module.declaration
        "#;

        // Reference/usage query - finds all symbol usages AND declarations
        let reference_query_str = r#"
        ; Function declarations (as reference points)
        (value_declaration
          functionDeclarationLeft: (function_declaration_left
            (lower_case_identifier) @ref.declaration))

        ; Function calls and references (via value_qid)
        (value_expr 
          name: (value_qid) @ref.function)

        ; Variable references (via lower_case_identifier within value_qid)
        (value_qid
          (lower_case_identifier) @ref.variable)

        ; Type references in type annotations
        (type_expression
          (type_ref
            (upper_case_qid) @ref.type))

        ; Type references in constructor patterns
        (pattern
          (union_pattern
            (upper_case_qid) @ref.constructor))

        ; Field access references
        (field_access_expr
          target: (_) @ref.target)

        ; Pattern variable references with 'as' keyword  
        (pattern
          patternAs: (lower_pattern) @ref.pattern_alias)

        ; Record base references in record expressions
        (record_base_identifier) @ref.record_base

        ; Record field references in field definitions
        (field
          name: (lower_case_identifier) @ref.record_field)
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

        let reference_query = Query::new(&language, reference_query_str)
            .map_err(|e| anyhow!("Failed to compile reference query: {}", e))?;

        Ok(Self {
            function_query,
            type_query,
            import_query,
            constant_query,
            module_query,
            reference_query,
            language,
        })
    }

    /// Extract all symbols from a Gren file
    pub fn extract_symbols(&self, uri: &Url, tree: &Tree, source: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let root_node = tree.root_node();

        // First, extract the module name to use as container for other symbols
        let module_name = self.extract_module_name(&root_node, source)?;

        // Extract functions
        symbols.extend(self.extract_functions(uri, &root_node, source, module_name.as_deref())?);

        // Extract types
        symbols.extend(self.extract_types(uri, &root_node, source, module_name.as_deref())?);

        // Extract constants
        symbols.extend(self.extract_constants(uri, &root_node, source, module_name.as_deref())?);

        // Extract module declaration
        symbols.extend(self.extract_module(uri, &root_node, source)?);

        debug!("Extracted {} symbols from {} with module container '{:?}'", symbols.len(), uri, module_name);
        Ok(symbols)
    }

    /// Extract all symbol references from a Gren file
    pub fn extract_references(&self, uri: &Url, tree: &Tree, source: &str) -> Result<Vec<SymbolReference>> {
        let mut references = Vec::new();
        let root_node = tree.root_node();
        
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.reference_query, root_node, source.as_bytes());

        for m in matches {
            for capture in m.captures {
                let capture_name = self.reference_query.capture_names()[capture.index as usize];
                let node_text = get_node_text(capture.node, source);
                
                // Skip empty or irrelevant references
                if node_text.trim().is_empty() || node_text.starts_with('_') {
                    continue;
                }

                // Determine reference kind based on capture name
                let reference_kind = match capture_name {
                    "ref.declaration" => ReferenceKind::Declaration,
                    "ref.function" | "ref.variable" | "ref.constructor" => ReferenceKind::Usage,
                    "ref.type" => ReferenceKind::Usage,
                    "ref.field" | "ref.target" | "ref.record_field" => ReferenceKind::Usage,
                    _ => ReferenceKind::Usage, // Default to usage
                };

                // Convert tree-sitter position to LSP range
                let start_pos = capture.node.start_position();
                let end_pos = capture.node.end_position();
                
                let range = Range {
                    start: Position {
                        line: start_pos.row as u32,
                        character: start_pos.column as u32,
                    },
                    end: Position {
                        line: end_pos.row as u32,
                        character: end_pos.column as u32,
                    },
                };

                let reference = SymbolReference::new(
                    node_text,
                    uri,
                    range,
                    reference_kind,
                );

                references.push(reference);
            }
        }

        debug!("Extracted {} references from {}", references.len(), uri);
        Ok(references)
    }

    /// Extract module name from the file
    fn extract_module_name(&self, node: &Node, source: &str) -> Result<Option<String>> {
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.module_query, *node, source.as_bytes());

        for m in matches {
            for capture in m.captures {
                let capture_name = self.module_query.capture_names()[capture.index as usize];
                if capture_name == "module.name" {
                    return Ok(Some(get_node_text(capture.node, source)));
                }
            }
        }
        Ok(None)
    }

    /// Extract function definitions
    fn extract_functions(&self, uri: &Url, node: &Node, source: &str, container: Option<&str>) -> Result<Vec<Symbol>> {
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
                        container.map(|s| s.to_string()), // Use module container
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
    fn extract_types(&self, uri: &Url, node: &Node, source: &str, container: Option<&str>) -> Result<Vec<Symbol>> {
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
                            container.map(|s| s.to_string()),
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
                            container.map(|s| s.to_string()),
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
    fn extract_constants(&self, uri: &Url, node: &Node, source: &str, container: Option<&str>) -> Result<Vec<Symbol>> {
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
                    container.map(|s| s.to_string()),
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
        let mut imports = std::collections::HashMap::new();
        let root_node = tree.root_node();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.import_query, root_node, source.as_bytes());

        // Collect all captures for each import clause node
        for m in matches {
            for capture in m.captures {
                let capture_name = self.import_query.capture_names()[capture.index as usize];
                
                // Find the import_clause parent node to group captures
                let import_clause_node = find_import_clause_parent(capture.node);
                let import_node_id = import_clause_node.id();
                
                let import_entry = imports.entry(import_node_id).or_insert_with(|| {
                    (String::new(), None, Vec::new(), false)
                });
                
                match capture_name {
                    "import.module" => {
                        import_entry.0 = get_node_text(capture.node, source);
                    }
                    "import.alias" => {
                        import_entry.1 = Some(get_node_text(capture.node, source));
                    }
                    "import.value" | "import.type" => {
                        import_entry.2.push(get_node_text(capture.node, source));
                    }
                    "import.all" => {
                        import_entry.3 = true;
                    }
                    _ => {}
                }
            }
        }

        // Convert to ImportInfo objects
        let import_list: Vec<ImportInfo> = imports.into_values()
            .filter(|(module_name, _, _, _)| !module_name.is_empty())
            .map(|(module_name, alias_name, imported_symbols, exposing_all)| {
                ImportInfo::new(
                    uri,
                    module_name,
                    if imported_symbols.is_empty() { None } else { Some(imported_symbols) },
                    alias_name,
                    exposing_all,
                )
            })
            .collect();

        debug!("Extracted {} imports from {}", import_list.len(), uri);
        Ok(import_list)
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
fn get_gren_language() -> Result<tree_sitter::Language> {
    gren_language::language()
}

/// Find the import_clause ancestor node for grouping captures
fn find_import_clause_parent(mut node: Node) -> Node {
    while let Some(parent) = node.parent() {
        if parent.kind() == "import_clause" {
            return parent;
        }
        node = parent;
    }
    node
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_compilation() {
        // Test that all queries compile successfully with the actual Gren grammar
        let result = GrenQueryEngine::new();
        
        // Should now succeed with the integrated grammar
        match result {
            Ok(engine) => {
                // Verify the engine was created successfully
                assert!(format!("{:?}", engine).contains("GrenQueryEngine"));
            }
            Err(e) => panic!("Query engine creation should succeed: {}", e),
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