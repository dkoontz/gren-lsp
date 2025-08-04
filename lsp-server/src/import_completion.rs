use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tracing::debug;

use crate::symbol_index::{Symbol, SymbolIndex, ImportInfo};
use crate::import_manager::{ImportManager, ImportVariant};
use crate::completion::CompletionContext;

/// Engine for proactive import completion - adds unimported symbols to completion results
pub struct ImportCompletionEngine {
    /// Symbol index for workspace-wide symbol access
    symbol_index: Arc<RwLock<Option<SymbolIndex>>>,
    /// Import manager for generating import TextEdits
    import_manager: Arc<ImportManager>,
}

/// Represents a completion item with automatic import
#[derive(Debug, Clone)]
pub struct ImportCompletionItem {
    /// The base completion item
    pub completion_item: CompletionItem,
    /// The import variant this completion represents
    pub import_variant: ImportVariant,
    /// The symbol being imported
    pub symbol: Symbol,
}

impl ImportCompletionEngine {
    /// Create new import completion engine
    pub fn new(
        symbol_index: Arc<RwLock<Option<SymbolIndex>>>,
        import_manager: Arc<ImportManager>,
    ) -> Self {
        Self {
            symbol_index,
            import_manager,
        }
    }

    /// Generate completion items for unimported symbols matching the prefix
    pub async fn complete_unimported_symbols(
        &self,
        context: &CompletionContext,
    ) -> Result<Vec<ImportCompletionItem>> {
        let start_time = std::time::Instant::now();
        let prefix = &context.word_prefix;
        
        debug!("Generating import completions for prefix: '{}'", prefix);

        // Get symbol index
        let symbol_index = self.symbol_index.read().await;
        let symbol_index = match symbol_index.as_ref() {
            Some(index) => index,
            None => {
                debug!("Symbol index not available for import completion");
                return Ok(Vec::new());
            }
        };

        // Get current file's import information
        let current_imports = symbol_index.get_imports_for_file(&context.uri).await?;
        
        // Find all workspace symbols matching prefix that are NOT already imported
        let workspace_symbols = symbol_index.find_workspace_symbols_by_prefix(prefix, 50).await?;
        let unimported_symbols = self.filter_unimported_symbols(workspace_symbols, &current_imports)?;

        debug!("Found {} unimported symbols matching prefix '{}'", unimported_symbols.len(), prefix);

        // Generate completion variants for each unimported symbol
        let mut completion_items = Vec::new();
        for symbol in unimported_symbols {
            let variants = self.generate_import_variants(&symbol, &current_imports, context).await?;
            completion_items.extend(variants);
        }

        let elapsed = start_time.elapsed();
        debug!("Generated {} import completion items in {:?}", completion_items.len(), elapsed);

        Ok(completion_items)
    }

    /// Filter out symbols that are already imported in the current file
    fn filter_unimported_symbols(
        &self,
        workspace_symbols: Vec<Symbol>,
        current_imports: &[ImportInfo],
    ) -> Result<Vec<Symbol>> {
        let mut unimported = Vec::new();

        for symbol in workspace_symbols {
            let mut is_already_imported = false;

            // Check if symbol is already available through existing imports
            for import in current_imports {
                if import.imported_module == symbol.container.as_deref().unwrap_or("") {
                    // Check if symbol is specifically imported or if module is imported with exposing all
                    if import.exposing_all || import.get_imported_symbols().contains(&symbol.name) {
                        is_already_imported = true;
                        break;
                    }
                }
            }

            if !is_already_imported {
                unimported.push(symbol);
            }
        }

        Ok(unimported)
    }

    /// Generate completion variants (exposed and qualified) for a symbol
    async fn generate_import_variants(
        &self,
        symbol: &Symbol,
        current_imports: &[ImportInfo],
        context: &CompletionContext,
    ) -> Result<Vec<ImportCompletionItem>> {
        let mut variants = Vec::new();

        let module_name = match &symbol.container {
            Some(container) => container,
            None => {
                debug!("Symbol '{}' has no container, skipping import variants", symbol.name);
                return Ok(variants);
            }
        };

        // Generate exposed import variant: import Module exposing (symbol)
        let exposed_variant = self.create_exposed_import_variant(symbol, module_name, current_imports, context).await?;
        variants.push(exposed_variant);

        // Generate qualified import variant: import Module + Module.symbol
        let qualified_variant = self.create_qualified_import_variant(symbol, module_name, current_imports, context).await?;
        variants.push(qualified_variant);

        Ok(variants)
    }

    /// Create an exposed import completion variant
    async fn create_exposed_import_variant(
        &self,
        symbol: &Symbol,
        module_name: &str,
        current_imports: &[ImportInfo],
        context: &CompletionContext,
    ) -> Result<ImportCompletionItem> {
        // Determine import strategy
        let strategy = self.import_manager.determine_import_strategy(
            module_name,
            &symbol.name,
            current_imports,
            ImportVariant::Exposed,
        ).await?;

        // Generate import TextEdit
        let additional_text_edits = self.import_manager.generate_import_edits(
            &context.content,
            &strategy,
        ).await?;

        // Create completion item
        let completion_item = CompletionItem {
            label: symbol.name.clone(),
            kind: Some(self.symbol_to_completion_kind(symbol)),
            detail: Some(self.format_symbol_detail(symbol, ImportVariant::Exposed, module_name)),
            documentation: self.create_import_documentation(symbol, ImportVariant::Exposed, module_name),
            insert_text: Some(symbol.name.clone()),
            additional_text_edits: Some(additional_text_edits),
            sort_text: Some(format!("1_{}", symbol.name)), // Prioritize exposed imports
            filter_text: Some(symbol.name.clone()),
            ..Default::default()
        };

        Ok(ImportCompletionItem {
            completion_item,
            import_variant: ImportVariant::Exposed,
            symbol: symbol.clone(),
        })
    }

    /// Create a qualified import completion variant
    async fn create_qualified_import_variant(
        &self,
        symbol: &Symbol,
        module_name: &str,
        current_imports: &[ImportInfo],
        context: &CompletionContext,
    ) -> Result<ImportCompletionItem> {
        let qualified_name = format!("{}.{}", module_name, symbol.name);

        // Determine import strategy  
        let strategy = self.import_manager.determine_import_strategy(
            module_name,
            &symbol.name,
            current_imports,
            ImportVariant::Qualified,
        ).await?;

        // Generate import TextEdit
        let additional_text_edits = self.import_manager.generate_import_edits(
            &context.content,
            &strategy,
        ).await?;

        // Create completion item
        let completion_item = CompletionItem {
            label: qualified_name.clone(),
            kind: Some(self.symbol_to_completion_kind(symbol)),
            detail: Some(self.format_symbol_detail(symbol, ImportVariant::Qualified, module_name)),
            documentation: self.create_import_documentation(symbol, ImportVariant::Qualified, module_name),
            insert_text: Some(qualified_name.clone()),
            additional_text_edits: Some(additional_text_edits),
            sort_text: Some(format!("2_{}", qualified_name)), // Lower priority than exposed
            filter_text: Some(qualified_name),
            ..Default::default()
        };

        Ok(ImportCompletionItem {
            completion_item,
            import_variant: ImportVariant::Qualified,
            symbol: symbol.clone(),
        })
    }

    /// Map symbol to appropriate completion item kind
    fn symbol_to_completion_kind(&self, symbol: &Symbol) -> CompletionItemKind {
        use crate::symbol_index::i32_to_symbol_kind;
        
        match i32_to_symbol_kind(symbol.kind) {
            SymbolKind::FUNCTION => CompletionItemKind::FUNCTION,
            SymbolKind::ENUM | SymbolKind::STRUCT => CompletionItemKind::CLASS,
            SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
            SymbolKind::MODULE => CompletionItemKind::MODULE,
            _ => CompletionItemKind::VALUE,
        }
    }

    /// Format detail text for import completion items
    fn format_symbol_detail(&self, symbol: &Symbol, variant: ImportVariant, module_name: &str) -> String {
        let type_info = symbol.signature.as_deref().unwrap_or("unknown");
        let import_info = match variant {
            ImportVariant::Exposed => format!("📦 import {} exposing ({})", module_name, symbol.name),
            ImportVariant::Qualified => format!("📦 import {}", module_name),
        };
        
        format!("{}\n{}", type_info, import_info)
    }

    /// Create documentation for import completion items
    fn create_import_documentation(&self, symbol: &Symbol, variant: ImportVariant, module_name: &str) -> Option<Documentation> {
        let type_sig = symbol.signature.as_deref().unwrap_or("unknown");
        let import_statement = match variant {
            ImportVariant::Exposed => format!("import {} exposing ({})", module_name, symbol.name),
            ImportVariant::Qualified => format!("import {}", module_name),
        };

        let doc_content = format!(
            "```gren\n{}\n```\n\nThis completion will automatically add:\n```gren\n{}\n```",
            type_sig, import_statement
        );

        Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc_content,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_filter_unimported_symbols() {
        // Test will be implemented with integration tests
        // This is a placeholder for the basic test structure
    }
}