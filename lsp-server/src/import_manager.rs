use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
// use tracing::{debug, trace};

use crate::symbol_index::ImportInfo;
use crate::tree_sitter_queries::GrenQueryEngine;

/// Manages import statement analysis and generation
pub struct ImportManager {
    /// Query engine for import analysis
    query_engine: GrenQueryEngine,
}

/// Import completion variants
#[derive(Debug, Clone, PartialEq)]
pub enum ImportVariant {
    /// Exposed import: import Module exposing (symbol)
    Exposed,
    /// Qualified import: import Module
    Qualified,
}

/// Strategy for adding imports to a file
#[derive(Debug, Clone)]
pub struct ImportStrategy {
    /// Type of import to add
    pub variant: ImportVariant,
    /// Module name to import
    pub module_name: String,
    /// Symbol name being imported (for exposed imports)
    pub symbol_name: String,
    /// Action to take
    pub action: ImportAction,
}

/// Actions for import management
#[derive(Debug, Clone)]
pub enum ImportAction {
    /// Add new import statement
    AddNew {
        /// Position to insert the import
        position: Position,
        /// Full import statement text
        import_text: String,
    },
    /// Extend existing exposing list
    ExtendExposing {
        /// Range of existing exposing list to replace
        range: Range,
        /// New exposing list text
        new_exposing_text: String,
    },
    /// Use existing qualified import (no changes needed)
    UseExisting,
}

impl ImportManager {
    /// Create new import manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            query_engine: GrenQueryEngine::new()?,
        })
    }

    /// Determine the best import strategy for a symbol
    pub async fn determine_import_strategy(
        &self,
        module_name: &str,
        symbol_name: &str,
        current_imports: &[ImportInfo],
        desired_variant: ImportVariant,
    ) -> Result<ImportStrategy> {
        // Check if module is already imported
        if let Some(existing_import) = current_imports.iter().find(|imp| imp.imported_module == module_name) {
            match desired_variant {
                ImportVariant::Exposed => {
                    if existing_import.exposing_all {
                        // Module already imports everything, symbol is already available
                        return Ok(ImportStrategy {
                            variant: ImportVariant::Exposed,
                            module_name: module_name.to_string(),
                            symbol_name: symbol_name.to_string(),
                            action: ImportAction::UseExisting,
                        });
                    } else if existing_import.get_imported_symbols().contains(&symbol_name.to_string()) {
                        // Symbol already in exposing list
                        return Ok(ImportStrategy {
                            variant: ImportVariant::Exposed,
                            module_name: module_name.to_string(),
                            symbol_name: symbol_name.to_string(),
                            action: ImportAction::UseExisting,
                        });
                    } else {
                        // Extend existing exposing list
                        let mut symbols = existing_import.get_imported_symbols();
                        symbols.push(symbol_name.to_string());
                        symbols.sort();
                        
                        let new_exposing_text = if symbols.len() == 1 {
                            format!("exposing ({})", symbols[0])
                        } else {
                            format!("exposing ({})", symbols.join(", "))
                        };

                        // This would need the range of the existing exposing clause
                        // For now, fall back to adding new import
                        return self.create_new_import_strategy(module_name, symbol_name, desired_variant, current_imports).await;
                    }
                }
                ImportVariant::Qualified => {
                    // Can reuse existing import for qualified access
                    return Ok(ImportStrategy {
                        variant: ImportVariant::Qualified,
                        module_name: module_name.to_string(),
                        symbol_name: symbol_name.to_string(),
                        action: ImportAction::UseExisting,
                    });
                }
            }
        }

        // No existing import, create new one
        self.create_new_import_strategy(module_name, symbol_name, desired_variant, current_imports).await
    }

    /// Create strategy for adding a new import statement
    async fn create_new_import_strategy(
        &self,
        module_name: &str,
        symbol_name: &str,
        variant: ImportVariant,
        current_imports: &[ImportInfo],
    ) -> Result<ImportStrategy> {
        let import_text = match variant {
            ImportVariant::Exposed => format!("import {} exposing ({})", module_name, symbol_name),
            ImportVariant::Qualified => format!("import {}", module_name),
        };

        // Determine position to insert import (after existing imports)
        let insert_position = self.calculate_import_position(current_imports);

        Ok(ImportStrategy {
            variant,
            module_name: module_name.to_string(),
            symbol_name: symbol_name.to_string(),
            action: ImportAction::AddNew {
                position: insert_position,
                import_text,
            },
        })
    }

    /// Calculate where to insert a new import statement
    fn calculate_import_position(&self, current_imports: &[ImportInfo]) -> Position {
        if current_imports.is_empty() {
            // Insert after module declaration (typically line 2)
            Position { line: 2, character: 0 }
        } else {
            // For now, insert after the last existing import (line 3-4 range)
            // TODO: Use actual line positions from tree-sitter parsing
            Position { line: 3 + current_imports.len() as u32, character: 0 }
        }
    }

    /// Generate TextEdit operations for import changes
    pub async fn generate_import_edits(
        &self,
        _document_content: &str,
        strategy: &ImportStrategy,
    ) -> Result<Vec<TextEdit>> {
        match &strategy.action {
            ImportAction::AddNew { position, import_text } => {
                Ok(vec![TextEdit {
                    range: Range {
                        start: *position,
                        end: *position,
                    },
                    new_text: format!("{}\n", import_text),
                }])
            }
            ImportAction::ExtendExposing { range, new_exposing_text } => {
                Ok(vec![TextEdit {
                    range: *range,
                    new_text: new_exposing_text.clone(),
                }])
            }
            ImportAction::UseExisting => {
                // No TextEdit needed
                Ok(Vec::new())
            }
        }
    }

    /// Analyze imports in a document to find optimal insertion points
    pub async fn analyze_imports(&self, document_content: &str, uri: &url::Url) -> Result<Vec<ImportInfo>> {
        // Parse document with tree-sitter
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_gren::language())
            .map_err(|e| anyhow!("Failed to set Gren language: {}", e))?;

        let tree = parser.parse(document_content, None)
            .ok_or_else(|| anyhow!("Failed to parse document"))?;

        // Use query engine to extract imports and convert to symbol_index ImportInfo
        let tree_imports = self.query_engine.extract_imports(uri, &tree, document_content)?;
        
        // Convert tree_sitter ImportInfo to symbol_index ImportInfo
        let imports = tree_imports.into_iter().map(|tree_import| {
            ImportInfo::new(
                uri,
                tree_import.imported_module,
                tree_import.imported_symbols,
                tree_import.alias_name,
                tree_import.exposing_all,
            )
        }).collect();

        Ok(imports)
    }

    /// Format import statement with proper spacing and ordering
    pub fn format_import_statement(&self, module_name: &str, exposing_symbols: &[String]) -> String {
        if exposing_symbols.is_empty() {
            format!("import {}", module_name)
        } else if exposing_symbols.len() == 1 {
            format!("import {} exposing ({})", module_name, exposing_symbols[0])
        } else {
            // Sort symbols alphabetically
            let mut sorted_symbols = exposing_symbols.to_vec();
            sorted_symbols.sort();
            format!("import {} exposing ({})", module_name, sorted_symbols.join(", "))
        }
    }

    /// Check if two imports can be merged (same module)
    pub fn can_merge_imports(&self, import1: &ImportInfo, import2: &ImportInfo) -> bool {
        import1.imported_module == import2.imported_module
    }

    /// Merge two import statements into one
    pub fn merge_imports(&self, import1: &ImportInfo, import2: &ImportInfo) -> Result<String> {
        if !self.can_merge_imports(import1, import2) {
            return Err(anyhow!("Cannot merge imports from different modules"));
        }

        let module_name = &import1.imported_module;
        
        // Combine exposing lists
        let mut all_symbols = Vec::new();
        all_symbols.extend(import1.get_imported_symbols());
        all_symbols.extend(import2.get_imported_symbols());
        
        // Remove duplicates and sort
        all_symbols.sort();
        all_symbols.dedup();

        Ok(self.format_import_statement(module_name, &all_symbols))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_import_statement() {
        let manager = ImportManager::new().unwrap();

        // Test simple import
        assert_eq!(
            manager.format_import_statement("Utils", &[]),
            "import Utils"
        );

        // Test single exposing
        assert_eq!(
            manager.format_import_statement("Utils", &["helper".to_string()]),
            "import Utils exposing (helper)"
        );

        // Test multiple exposing (should be sorted)
        assert_eq!(
            manager.format_import_statement("Utils", &["zzz".to_string(), "aaa".to_string(), "mmm".to_string()]),
            "import Utils exposing (aaa, mmm, zzz)"
        );
    }

    #[test]
    fn test_calculate_import_position() {
        let manager = ImportManager::new().unwrap();

        // Test empty imports
        assert_eq!(
            manager.calculate_import_position(&[]),
            Position { line: 2, character: 0 }
        );

        // Test with existing imports
        let test_uri = url::Url::parse("file:///test.gren").unwrap();
        let imports = vec![
            ImportInfo::new(
                &test_uri,
                "Utils".to_string(),
                Some(vec!["helper".to_string()]),
                None,
                false,
            )
        ];
        
        assert_eq!(
            manager.calculate_import_position(&imports),
            Position { line: 4, character: 0 }
        );
    }
}