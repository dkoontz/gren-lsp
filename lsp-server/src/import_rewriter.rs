use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Parser, Query, QueryCursor};
use tracing::debug;

use crate::tree_sitter_queries::GrenQueryEngine;

/// Engine for rewriting import statements when modules are renamed
pub struct ImportRewriter {
    /// Tree-sitter parser for Gren
    parser: std::sync::Mutex<Parser>,
    /// Query engine for import analysis
    query_engine: GrenQueryEngine,
}

impl ImportRewriter {
    /// Create new import rewriter
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_gren::language())
            .map_err(|e| anyhow!("Failed to set Gren language: {}", e))?;

        let query_engine = GrenQueryEngine::new()?;

        Ok(Self {
            parser: std::sync::Mutex::new(parser),
            query_engine,
        })
    }

    /// Check if a document contains imports referencing a specific module
    pub async fn has_import_reference(&self, content: &str, module_name: &str) -> Result<bool> {
        let tree = self.parser.lock().unwrap().parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse document"))?;

        // Use the existing query engine to extract imports
        let uri = url::Url::parse("file:///temp.gren")?;
        let imports = self.query_engine.extract_imports(&uri, &tree, content)?;

        // Check if any import references the target module
        for import in imports {
            if import.imported_module == module_name {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Rewrite import statements in a document to use the new module name
    pub async fn rewrite_imports(
        &self,
        content: &str,
        old_module_name: &str,
        new_module_name: &str,
    ) -> Result<Vec<TextEdit>> {
        debug!("Rewriting imports: {} -> {}", old_module_name, new_module_name);

        let tree = self.parser.lock().unwrap().parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse document for import rewriting"))?;

        // Use the existing query engine to extract imports
        let uri = url::Url::parse("file:///temp.gren")?;
        let imports = self.query_engine.extract_imports(&uri, &tree, content)?;

        let mut edits = Vec::new();

        // Find imports that reference the old module name
        for import in imports {
            if import.imported_module == old_module_name {
                // Find the import statement in the tree using tree-sitter queries
                let import_query = tree_sitter::Query::new(
                    &tree_sitter_gren::language(),
                    r#"
                    (import_clause
                      moduleName: (upper_case_qid) @module.name) @import.clause
                    "#,
                ).map_err(|e| anyhow!("Failed to compile import query: {}", e))?;

                let mut cursor = tree_sitter::QueryCursor::new();
                let matches = cursor.matches(&import_query, tree.root_node(), content.as_bytes());

                for m in matches {
                    for capture in m.captures {
                        let capture_name = import_query.capture_names()[capture.index as usize];
                        
                        if capture_name == "module.name" {
                            let node_text = capture.node.utf8_text(content.as_bytes())
                                .map_err(|e| anyhow!("Failed to get node text: {}", e))?;
                            
                            // Check if this is the module we want to rename
                            if node_text == old_module_name {
                                let start_pos = self.ts_position_to_lsp_position(capture.node.start_position(), content)?;
                                let end_pos = self.ts_position_to_lsp_position(capture.node.end_position(), content)?;

                                let edit = TextEdit {
                                    range: Range {
                                        start: start_pos,
                                        end: end_pos,
                                    },
                                    new_text: new_module_name.to_string(),
                                };
                                edits.push(edit);
                            }
                        }
                    }
                }
            }
        }

        debug!("Generated {} import edits using tree-sitter", edits.len());
        Ok(edits)
    }

    /// Convert tree-sitter position to LSP position
    fn ts_position_to_lsp_position(&self, ts_pos: tree_sitter::Point, content: &str) -> Result<Position> {
        // Convert UTF-8 byte offset to UTF-16 code unit offset for LSP
        let lines: Vec<&str> = content.lines().collect();
        
        if ts_pos.row >= lines.len() {
            return Err(anyhow!("Position row {} out of bounds", ts_pos.row));
        }

        let line = lines[ts_pos.row];
        if ts_pos.column > line.len() {
            return Err(anyhow!("Position column {} out of bounds for line {}", ts_pos.column, ts_pos.row));
        }

        // Calculate UTF-16 offset for the column
        let line_prefix = &line[..ts_pos.column];
        let utf16_column = line_prefix.encode_utf16().count() as u32;

        Ok(Position {
            line: ts_pos.row as u32,
            character: utf16_column,
        })
    }

    /// Update module declaration in a file that was renamed
    pub async fn update_module_declaration(
        &self,
        content: &str,
        new_module_name: &str,
    ) -> Result<Vec<TextEdit>> {
        debug!("Updating module declaration to: {}", new_module_name);

        let tree = self.parser.lock().unwrap().parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse document for module declaration update"))?;

        let mut edits = Vec::new();

        // Find module declaration using tree-sitter query
        let module_query = tree_sitter::Query::new(
            &tree_sitter_gren::language(),
            r#"
            (module_declaration
              name: (upper_case_qid) @module.name) @module.declaration
            "#,
        ).map_err(|e| anyhow!("Failed to compile module query: {}", e))?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(&module_query, tree.root_node(), content.as_bytes());

        for m in matches {
            for capture in m.captures {
                let capture_name = module_query.capture_names()[capture.index as usize];
                
                if capture_name == "module.name" {
                    let start_pos = self.ts_position_to_lsp_position(capture.node.start_position(), content)?;
                    let end_pos = self.ts_position_to_lsp_position(capture.node.end_position(), content)?;

                    let edit = TextEdit {
                        range: Range {
                            start: start_pos,
                            end: end_pos,
                        },
                        new_text: new_module_name.to_string(),
                    };
                    edits.push(edit);
                    break; // Should only be one module declaration
                }
            }
        }

        debug!("Generated {} module declaration edits using tree-sitter", edits.len());
        Ok(edits)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_has_import_reference() {
        let rewriter = ImportRewriter::new().unwrap();

        // Test simple import
        let content = "import Utils\n\nmain = Utils.helper \"test\"";
        let result = rewriter.has_import_reference(content, "Utils").await.unwrap();
        assert!(result);

        let result = rewriter.has_import_reference(content, "Other").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_rewrite_imports() {
        let rewriter = ImportRewriter::new().unwrap();

        // Test simple import replacement
        let content = "module Main exposing (..)\n\nimport Utils\nimport Other\n\nmain = Utils.helper \"test\"";
        let edits = rewriter.rewrite_imports(content, "Utils", "Helpers").await.unwrap();
        
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "import Helpers");
        assert_eq!(edits[0].range.start.line, 2); // Third line (0-indexed)
    }

    #[tokio::test]
    async fn test_update_module_declaration() {
        let rewriter = ImportRewriter::new().unwrap();

        // Test module declaration update
        let content = "module Utils exposing (..)\n\nhelper : String -> String\nhelper s = s";
        let edits = rewriter.update_module_declaration(content, "Helpers").await.unwrap();
        
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "module Helpers exposing (..)");
        assert_eq!(edits[0].range.start.line, 0); // First line
    }

    #[tokio::test]
    async fn test_rewrite_imports_with_alias() {
        let rewriter = ImportRewriter::new().unwrap();

        // Test import with alias
        let content = "import Utils as U\nimport Http.Client\n\nmain = U.helper (Client.get url)";
        let edits = rewriter.rewrite_imports(content, "Utils", "Helpers").await.unwrap();
        
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "import Helpers as U");
    }

    #[tokio::test]
    async fn test_rewrite_imports_nested_module() {
        let rewriter = ImportRewriter::new().unwrap();

        // Test nested module replacement
        let content = "import Http.Client\nimport Utils\n\nmain = Client.get url";
        let edits = rewriter.rewrite_imports(content, "Http.Client", "Network.Http").await.unwrap();
        
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "import Network.Http");
    }
}