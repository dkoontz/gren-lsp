use anyhow::{Context, Result};
use lsp_types::*;
use rusqlite::{Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::debug;
use tree_sitter::{Query, QueryCursor, Tree};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container_name: Option<String>,
    pub type_signature: Option<String>,
    pub documentation: Option<String>,
}

pub struct SymbolIndex {
    connection: Arc<Mutex<Connection>>,
}

impl SymbolIndex {
    pub fn new() -> Result<Self> {
        // TODO: Use proper data directory
        let db_path = PathBuf::from("gren-lsp-symbols.db");
        let connection = Connection::open(db_path)?;

        // Create tables
        connection.execute(
            "CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                file_uri TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                start_character INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                end_character INTEGER NOT NULL,
                container_name TEXT,
                type_signature TEXT,
                documentation TEXT
            )",
            [],
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name)",
            [],
        )?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn index_symbol(&self, symbol: &Symbol) -> SqlResult<()> {
        let connection = self.connection.lock().unwrap();
        connection.execute(
            "INSERT OR REPLACE INTO symbols 
            (name, kind, file_uri, start_line, start_character, end_line, end_character, 
             container_name, type_signature, documentation)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            [
                &symbol.name,
                &format!("{:?}", symbol.kind),
                &symbol.location.uri.to_string(),
                &symbol.location.range.start.line.to_string(),
                &symbol.location.range.start.character.to_string(),
                &symbol.location.range.end.line.to_string(),
                &symbol.location.range.end.character.to_string(),
                symbol.container_name.as_ref().unwrap_or(&String::new()),
                symbol.type_signature.as_ref().unwrap_or(&String::new()),
                symbol.documentation.as_ref().unwrap_or(&String::new()),
            ],
        )?;
        Ok(())
    }

    pub fn find_symbol(&self, name: &str) -> SqlResult<Vec<Symbol>> {
        let connection = self.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "SELECT name, kind, file_uri, start_line, start_character, end_line, end_character, 
             container_name, type_signature, documentation
             FROM symbols WHERE name LIKE ?1",
        )?;

        let symbol_iter = stmt.query_map([format!("%{}%", name)], |row| {
            let uri = Url::parse(&row.get::<_, String>(2)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            let kind_str: String = row.get(1)?;
            let kind = match kind_str.as_str() {
                "Function" => SymbolKind::FUNCTION,
                "Constructor" => SymbolKind::CONSTRUCTOR,
                "Module" => SymbolKind::MODULE,
                "Class" => SymbolKind::CLASS,
                "Variable" => SymbolKind::VARIABLE,
                "Field" => SymbolKind::FIELD,
                _ => SymbolKind::VARIABLE,
            };

            Ok(Symbol {
                name: row.get(0)?,
                kind,
                location: Location::new(
                    uri,
                    Range::new(
                        Position::new(row.get(3)?, row.get(4)?),
                        Position::new(row.get(5)?, row.get(6)?),
                    ),
                ),
                container_name: {
                    let container: String = row.get(7)?;
                    if container.is_empty() {
                        None
                    } else {
                        Some(container)
                    }
                },
                type_signature: {
                    let sig: String = row.get(8)?;
                    if sig.is_empty() {
                        None
                    } else {
                        Some(sig)
                    }
                },
                documentation: {
                    let doc: String = row.get(9)?;
                    if doc.is_empty() {
                        None
                    } else {
                        Some(doc)
                    }
                },
            })
        })?;

        let mut symbols = Vec::new();
        for symbol in symbol_iter {
            symbols.push(symbol?);
        }
        Ok(symbols)
    }

    pub fn clear_file_symbols(&self, file_uri: &str) -> SqlResult<()> {
        let connection = self.connection.lock().unwrap();
        connection.execute("DELETE FROM symbols WHERE file_uri = ?1", [file_uri])?;
        Ok(())
    }
}

/// Extracts symbols from a parsed Gren syntax tree
pub struct SymbolExtractor {
    function_query: Query,
    type_query: Query,
    constructor_query: Query,
    module_query: Query,
    #[allow(dead_code)]
    import_query: Query,
}

impl SymbolExtractor {
    pub fn new() -> Result<Self> {
        let language = crate::parser::Parser::language();

        // Query for function definitions and type annotations (top-level only)
        let function_query = Query::new(
            language,
            r#"
            ; Top-level function value declarations
            (file 
                (value_declaration 
                    (function_declaration_left 
                        (lower_case_identifier) @function.name)))
            
            ; Top-level function type annotations  
            (file
                (type_annotation 
                    (lower_case_identifier) @function.type_name
                    (colon)
                    (type_expression) @function.type_sig))
        "#,
        )
        .context("Failed to create function query")?;

        // Query for type definitions and aliases
        let type_query = Query::new(
            language,
            r#"
            ; Type declarations (union types)
            (type_declaration 
                (upper_case_identifier) @type.name) @type.definition
            
            ; Type aliases
            (type_alias_declaration 
                (upper_case_identifier) @type.name) @type.alias
        "#,
        )
        .context("Failed to create type query")?;

        // Query for union type constructors
        let constructor_query = Query::new(
            language,
            r#"
            ; Union type constructors with parent type
            (type_declaration 
                (upper_case_identifier) @constructor.parent_type
                (union_variant 
                    (upper_case_identifier) @constructor.name)) @constructor.definition
        "#,
        )
        .context("Failed to create constructor query")?;

        // Query for module declarations
        let module_query = Query::new(
            language,
            r#"
            ; Module declarations
            (module_declaration 
                (upper_case_qid 
                    (upper_case_identifier) @module.name)) @module.definition
        "#,
        )
        .context("Failed to create module query")?;

        // Query for import statements
        let import_query = Query::new(
            language,
            r#"
            ; Import statements
            (import_clause 
                (upper_case_qid 
                    (upper_case_identifier) @import.name)) @import.reference
        "#,
        )
        .context("Failed to create import query")?;

        Ok(Self {
            function_query,
            type_query,
            constructor_query,
            module_query,
            import_query,
        })
    }

    /// Extract all symbols from a parsed tree
    pub fn extract_symbols(
        &self,
        tree: &Tree,
        source: &str,
        file_uri: &Url,
    ) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut cursor = QueryCursor::new();

        // First, extract all documentation comments and associate them with their line positions
        let doc_comments = self.extract_documentation_comments(tree, source)?;

        // Extract functions with documentation
        symbols.extend(self.extract_functions(
            tree,
            source,
            file_uri,
            &mut cursor,
            &doc_comments,
        )?);

        // Extract types with documentation
        symbols.extend(self.extract_types(tree, source, file_uri, &mut cursor, &doc_comments)?);

        // Extract constructors with documentation
        symbols.extend(self.extract_constructors(
            tree,
            source,
            file_uri,
            &mut cursor,
            &doc_comments,
        )?);

        // Extract modules with documentation
        symbols.extend(self.extract_modules(tree, source, file_uri, &mut cursor, &doc_comments)?);

        debug!("Extracted {} symbols from {}", symbols.len(), file_uri);
        Ok(symbols)
    }

    /// Extract documentation comments from the source tree
    /// Returns a map of line numbers to documentation text
    fn extract_documentation_comments(
        &self,
        tree: &Tree,
        source: &str,
    ) -> Result<std::collections::HashMap<u32, String>> {
        let mut doc_comments = std::collections::HashMap::new();
        let language = crate::parser::Parser::language();

        // Query for block comments that are documentation comments
        let doc_query = Query::new(
            language,
            r#"
            (block_comment) @doc.comment
        "#,
        )
        .context("Failed to create documentation query")?;

        let mut cursor = QueryCursor::new();
        let source_bytes = source.as_bytes();
        let matches = cursor.matches(&doc_query, tree.root_node(), source_bytes);

        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                if let Ok(comment_text) = node.utf8_text(source_bytes) {
                    // Check if this is a documentation comment (starts with {-|)
                    if comment_text.starts_with("{-|") && comment_text.ends_with("-}") {
                        // Extract the inner documentation text
                        let inner_text = &comment_text[3..comment_text.len() - 2];
                        let cleaned_doc = Self::clean_documentation_text(inner_text);

                        // Associate with the line where the comment ends
                        let end_line = node.end_position().row as u32;
                        let preview =
                            cleaned_doc[..std::cmp::min(50, cleaned_doc.len())].to_string();
                        doc_comments.insert(end_line, cleaned_doc);

                        debug!(
                            "Found documentation comment ending at line {}: {}",
                            end_line, preview
                        );
                    }
                }
            }
        }

        debug!("Extracted {} documentation comments", doc_comments.len());
        Ok(doc_comments)
    }

    /// Clean documentation text by removing leading/trailing whitespace and normalizing formatting
    fn clean_documentation_text(doc_text: &str) -> String {
        doc_text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty()) // Remove empty lines
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// Find the documentation comment associated with a symbol at the given line
    /// Documentation comments should appear immediately before the symbol definition
    fn find_documentation_for_symbol(
        &self,
        symbol_line: u32,
        doc_comments: &std::collections::HashMap<u32, String>,
    ) -> Option<String> {
        // Look for documentation comments ending 1-2 lines before the symbol
        // This accounts for the fact that there might be a blank line between doc and symbol
        for offset in 1..=3 {
            if let Some(doc) = doc_comments.get(&(symbol_line.saturating_sub(offset))) {
                return Some(doc.clone());
            }
        }
        None
    }

    fn extract_functions(
        &self,
        tree: &Tree,
        source: &str,
        file_uri: &Url,
        cursor: &mut QueryCursor,
        doc_comments: &std::collections::HashMap<u32, String>,
    ) -> Result<Vec<Symbol>> {
        let mut functions = Vec::new();
        let source_bytes = source.as_bytes();

        // First pass: collect all function definitions and type annotations
        let matches = cursor.matches(&self.function_query, tree.root_node(), source_bytes);
        let mut function_defs: std::collections::HashMap<String, (Range, Option<String>)> =
            std::collections::HashMap::new();
        let mut type_annotations: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                if let Ok(text) = node.utf8_text(source_bytes) {
                    let capture_name = &self.function_query.capture_names()[capture.index as usize];

                    match capture_name.as_str() {
                        "function.name" => {
                            // This is a function definition
                            let range = Range::new(
                                Position::new(
                                    node.start_position().row as u32,
                                    node.start_position().column as u32,
                                ),
                                Position::new(
                                    node.end_position().row as u32,
                                    node.end_position().column as u32,
                                ),
                            );
                            function_defs.insert(text.to_string(), (range, None));
                        }
                        "function.type_name" => {
                            // This is the name in a type annotation - we'll look for the matching type signature
                            let func_name = text.to_string();

                            // Find the type signature for this function
                            for capture2 in m.captures {
                                let capture2_name =
                                    &self.function_query.capture_names()[capture2.index as usize];
                                if capture2_name == "function.type_sig" {
                                    if let Ok(type_text) = capture2.node.utf8_text(source_bytes) {
                                        type_annotations.insert(
                                            func_name.clone(),
                                            Self::clean_type_signature(type_text),
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Second pass: combine function definitions with their type annotations and documentation
        for (func_name, (range, _)) in function_defs {
            let type_signature = type_annotations.get(&func_name).cloned();
            let documentation = self.find_documentation_for_symbol(range.start.line, doc_comments);

            functions.push(Symbol {
                name: func_name,
                kind: SymbolKind::FUNCTION,
                location: Location::new(file_uri.clone(), range),
                container_name: None, // TODO: Extract containing module
                type_signature,
                documentation,
            });
        }

        debug!("Extracted {} functions", functions.len());
        Ok(functions)
    }

    fn extract_types(
        &self,
        tree: &Tree,
        source: &str,
        file_uri: &Url,
        cursor: &mut QueryCursor,
        doc_comments: &std::collections::HashMap<u32, String>,
    ) -> Result<Vec<Symbol>> {
        let mut types = Vec::new();
        let source_bytes = source.as_bytes();

        let matches = cursor.matches(&self.type_query, tree.root_node(), source_bytes);

        for m in matches {
            for capture in m.captures {
                let node = capture.node;

                if let Ok(name) = node.utf8_text(source_bytes) {
                    let range = Range::new(
                        Position::new(
                            node.start_position().row as u32,
                            node.start_position().column as u32,
                        ),
                        Position::new(
                            node.end_position().row as u32,
                            node.end_position().column as u32,
                        ),
                    );

                    let documentation =
                        self.find_documentation_for_symbol(range.start.line, doc_comments);

                    types.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::CLASS, // Using CLASS for type definitions
                        location: Location::new(file_uri.clone(), range),
                        container_name: None,
                        type_signature: None,
                        documentation,
                    });
                }
            }
        }

        debug!("Extracted {} types", types.len());
        Ok(types)
    }

    fn extract_constructors(
        &self,
        tree: &Tree,
        source: &str,
        file_uri: &Url,
        cursor: &mut QueryCursor,
        doc_comments: &std::collections::HashMap<u32, String>,
    ) -> Result<Vec<Symbol>> {
        let mut constructors = Vec::new();
        let source_bytes = source.as_bytes();

        let matches = cursor.matches(&self.constructor_query, tree.root_node(), source_bytes);

        for m in matches {
            let mut constructor_name: Option<String> = None;
            let mut parent_type: Option<String> = None;
            let mut constructor_range: Option<Range> = None;

            // Collect both constructor name and parent type from the same match
            for capture in m.captures {
                let node = capture.node;
                let capture_name = &self.constructor_query.capture_names()[capture.index as usize];

                if let Ok(text) = node.utf8_text(source_bytes) {
                    match capture_name.as_str() {
                        "constructor.name" => {
                            constructor_name = Some(text.to_string());

                            let range = Range::new(
                                Position::new(
                                    node.start_position().row as u32,
                                    node.start_position().column as u32,
                                ),
                                Position::new(
                                    node.end_position().row as u32,
                                    node.end_position().column as u32,
                                ),
                            );
                            constructor_range = Some(range);
                        }
                        "constructor.parent_type" => {
                            parent_type = Some(text.to_string());
                        }
                        _ => {}
                    }
                }
            }

            // Only create constructor symbol if we have both name and parent type
            if let (Some(name), Some(parent), Some(range)) =
                (constructor_name, parent_type, constructor_range)
            {
                let documentation =
                    self.find_documentation_for_symbol(range.start.line, doc_comments);

                constructors.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::CONSTRUCTOR,
                    location: Location::new(file_uri.clone(), range),
                    container_name: Some(parent.clone()),
                    type_signature: None,
                    documentation,
                });

                debug!("Found constructor '{}' for type '{}'", name, parent);
            }
        }

        debug!("Extracted {} constructors", constructors.len());
        Ok(constructors)
    }

    fn extract_modules(
        &self,
        tree: &Tree,
        source: &str,
        file_uri: &Url,
        cursor: &mut QueryCursor,
        doc_comments: &std::collections::HashMap<u32, String>,
    ) -> Result<Vec<Symbol>> {
        let mut modules = Vec::new();
        let source_bytes = source.as_bytes();

        let matches = cursor.matches(&self.module_query, tree.root_node(), source_bytes);

        for m in matches {
            let mut module_name: Option<String> = None;
            let mut module_range: Option<Range> = None;

            // Only capture the module name, not the full declaration
            for capture in m.captures {
                let node = capture.node;
                let capture_name = &self.module_query.capture_names()[capture.index as usize];

                if let Ok(text) = node.utf8_text(source_bytes) {
                    match capture_name.as_str() {
                        "module.name" => {
                            module_name = Some(text.to_string());

                            let range = Range::new(
                                Position::new(
                                    node.start_position().row as u32,
                                    node.start_position().column as u32,
                                ),
                                Position::new(
                                    node.end_position().row as u32,
                                    node.end_position().column as u32,
                                ),
                            );
                            module_range = Some(range);
                        }
                        _ => {
                            // Ignore module.definition capture - we only want the name
                        }
                    }
                }
            }

            // Only create module symbol if we have a name
            if let (Some(name), Some(range)) = (module_name, module_range) {
                let documentation =
                    self.find_documentation_for_symbol(range.start.line, doc_comments);

                modules.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::MODULE,
                    location: Location::new(file_uri.clone(), range),
                    container_name: None,
                    type_signature: None,
                    documentation,
                });

                debug!("Found module '{}'", name);
            }
        }

        debug!("Extracted {} modules", modules.len());
        Ok(modules)
    }

    /// Clean up type signature text by removing extra whitespace and formatting
    fn clean_type_signature(sig: &str) -> String {
        sig.lines()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_symbol_extraction_comprehensive() {
        let gren_source = r#"
module Utils.Math exposing (add, multiply, Point, Shape(..))

{-| A point in 2D space -}
type alias Point = 
    { x : Float
    , y : Float
    }

{-| Different shapes -}
type Shape 
    = Circle Float Point
    | Rectangle Float Float Point
    | Triangle Point Point Point

{-| Add two numbers -}
add : Int -> Int -> Int
add x y = x + y

{-| Multiply two numbers -}
multiply : Float -> Float -> Float  
multiply a b = a * b

{-| Calculate distance -}
distance : Point -> Point -> Float
distance p1 p2 = 
    let
        dx = p1.x - p2.x
        dy = p1.y - p2.y
    in
    sqrt (dx * dx + dy * dy)

{-| No type annotation function -}
simple x = x + 1
"#;

        // Create parser and extractor
        let mut parser = Parser::new().expect("Failed to create parser");
        let extractor = SymbolExtractor::new().expect("Failed to create extractor");

        // Parse the source
        let tree = parser
            .parse(gren_source)
            .expect("Failed to parse")
            .expect("No tree returned");

        // Extract symbols
        let file_uri = Url::parse("file:///utils.gren").expect("Invalid URI");
        let symbols = extractor
            .extract_symbols(&tree, gren_source, &file_uri)
            .expect("Failed to extract symbols");

        // Group symbols by kind for easier testing
        let functions: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::FUNCTION)
            .collect();
        let types: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::CLASS)
            .collect();
        let constructors: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::CONSTRUCTOR)
            .collect();
        let modules: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::MODULE)
            .collect();

        println!("=== FUNCTIONS ===");
        for func in &functions {
            println!("  {} -> {:?}", func.name, func.type_signature);
        }

        println!("=== TYPES ===");
        for typ in &types {
            println!("  {}", typ.name);
        }

        println!("=== CONSTRUCTORS ===");
        for cons in &constructors {
            println!("  {}", cons.name);
        }

        println!("=== MODULES ===");
        for module in &modules {
            println!("  {}", module.name);
        }

        // Test function extraction with type signatures
        assert_eq!(functions.len(), 4, "Should find 4 top-level functions");

        let add_func = functions
            .iter()
            .find(|f| f.name == "add")
            .expect("Should find add function");
        assert_eq!(
            add_func.type_signature,
            Some("Int -> Int -> Int".to_string())
        );

        let multiply_func = functions
            .iter()
            .find(|f| f.name == "multiply")
            .expect("Should find multiply function");
        assert_eq!(
            multiply_func.type_signature,
            Some("Float -> Float -> Float".to_string())
        );

        let distance_func = functions
            .iter()
            .find(|f| f.name == "distance")
            .expect("Should find distance function");
        assert_eq!(
            distance_func.type_signature,
            Some("Point -> Point -> Float".to_string())
        );

        // Test function without type annotation
        let simple_func = functions
            .iter()
            .find(|f| f.name == "simple")
            .expect("Should find simple function");
        assert_eq!(
            simple_func.type_signature, None,
            "simple function should have no type signature"
        );

        // Test type extraction
        assert!(
            types.iter().any(|t| t.name == "Point"),
            "Should find Point type"
        );
        assert!(
            types.iter().any(|t| t.name == "Shape"),
            "Should find Shape type"
        );

        // Test constructor extraction
        assert!(
            constructors.iter().any(|c| c.name == "Circle"),
            "Should find Circle constructor"
        );
        assert!(
            constructors.iter().any(|c| c.name == "Rectangle"),
            "Should find Rectangle constructor"
        );
        assert!(
            constructors.iter().any(|c| c.name == "Triangle"),
            "Should find Triangle constructor"
        );

        // Test module extraction
        assert!(
            modules.iter().any(|m| m.name == "Utils"),
            "Should find Utils module"
        );
    }

    #[test]
    fn test_symbol_extraction_basic() {
        let gren_source = r#"
module Main exposing (..)

type alias User = 
    { name : String
    , age : Int
    }

type Status = Active | Inactive

length : String -> Int
length str = String.length str

main : Program () Model Msg
main = Browser.sandbox { init = init, update = update, view = view }
"#;

        // Create parser and extractor
        let mut parser = Parser::new().expect("Failed to create parser");
        let extractor = SymbolExtractor::new().expect("Failed to create extractor");

        // Parse the source
        let tree = parser
            .parse(gren_source)
            .expect("Failed to parse")
            .expect("No tree returned");

        // Extract symbols
        let file_uri = Url::parse("file:///test.gren").expect("Invalid URI");
        let symbols = extractor
            .extract_symbols(&tree, gren_source, &file_uri)
            .expect("Failed to extract symbols");

        println!("Extracted {} symbols:", symbols.len());
        for symbol in &symbols {
            println!(
                "  {} ({:?}) at {:?}",
                symbol.name, symbol.kind, symbol.location.range
            );
            if let Some(sig) = &symbol.type_signature {
                println!("    Type: {}", sig);
            }
        }

        // Verify we found expected symbols
        let symbol_names: Vec<&String> = symbols.iter().map(|s| &s.name).collect();

        // Should find module
        assert!(
            symbol_names.contains(&&"Main".to_string()),
            "Should find Main module"
        );

        // Should find types
        assert!(
            symbol_names.contains(&&"User".to_string()),
            "Should find User type alias"
        );
        assert!(
            symbol_names.contains(&&"Status".to_string()),
            "Should find Status type"
        );

        // Should find constructors
        assert!(
            symbol_names.contains(&&"Active".to_string()),
            "Should find Active constructor"
        );
        assert!(
            symbol_names.contains(&&"Inactive".to_string()),
            "Should find Inactive constructor"
        );

        // Should find functions
        assert!(
            symbol_names.contains(&&"length".to_string()),
            "Should find length function"
        );
        assert!(
            symbol_names.contains(&&"main".to_string()),
            "Should find main function"
        );

        // Verify function has type signature
        let length_symbol = symbols
            .iter()
            .find(|s| s.name == "length")
            .expect("Should find length function");
        assert!(
            length_symbol.type_signature.is_some(),
            "length function should have type signature"
        );
    }

    #[test]
    fn test_symbol_index_operations() {
        let index = SymbolIndex::new().expect("Failed to create symbol index");
        let file_uri = Url::parse("file:///test.gren").expect("Invalid URI");

        // Create a test symbol
        let symbol = Symbol {
            name: "testFunction".to_string(),
            kind: SymbolKind::FUNCTION,
            location: Location::new(
                file_uri.clone(),
                Range::new(Position::new(5, 0), Position::new(5, 12)),
            ),
            container_name: None,
            type_signature: Some("String -> Int".to_string()),
            documentation: None,
        };

        // Index the symbol
        index.index_symbol(&symbol).expect("Failed to index symbol");

        // Search for the symbol
        let found = index.find_symbol("testFunction").expect("Failed to search");
        assert_eq!(found.len(), 1, "Should find exactly one symbol");
        assert_eq!(found[0].name, "testFunction");
        assert_eq!(found[0].type_signature, Some("String -> Int".to_string()));

        // Clear symbols for file
        index
            .clear_file_symbols(file_uri.as_str())
            .expect("Failed to clear symbols");

        // Verify symbols were cleared
        let found_after_clear = index
            .find_symbol("testFunction")
            .expect("Failed to search after clear");
        assert_eq!(
            found_after_clear.len(),
            0,
            "Should find no symbols after clearing"
        );
    }
}
