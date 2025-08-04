#[cfg(test)]
mod import_completion_integration_tests {
    use anyhow::Result;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tower_lsp::lsp_types::*;
    use url::Url;

    use crate::import_completion::ImportCompletionEngine;
    use crate::import_manager::ImportManager;
    use crate::symbol_index::SymbolIndex;
    use crate::completion::CompletionContext;

    async fn setup_test_workspace() -> Result<SymbolIndex> {
        // Use in-memory database for testing
        let workspace_root = std::env::temp_dir().join("import-completion-test");
        std::fs::create_dir_all(&workspace_root)?;
        
        let symbol_index = SymbolIndex::new_in_memory(workspace_root).await?;
        
        // Index a sample module with symbols
        let utils_uri = Url::parse("file:///workspace/Utils.gren")?;
        let utils_symbols = vec![
            crate::symbol_index::Symbol {
                id: None,
                name: "helper".to_string(),
                kind: 12, // Function
                signature: Some("helper : String -> String".to_string()),
                documentation: None,
                range_start_line: 5,
                range_start_char: 0,
                range_end_line: 5,
                range_end_char: 6,
                uri: utils_uri.to_string(),
                container: Some("Utils".to_string()),
                created_at: None,
            },
            crate::symbol_index::Symbol {
                id: None,
                name: "processor".to_string(),
                kind: 12, // Function
                signature: Some("processor : Array String -> Array String".to_string()),
                documentation: None,
                range_start_line: 8,
                range_start_char: 0,
                range_end_line: 8,
                range_end_char: 9,
                uri: utils_uri.to_string(),
                container: Some("Utils".to_string()),
                created_at: None,
            },
        ];
        
        symbol_index.update_symbols_for_file(&utils_uri, &utils_symbols).await?;
        
        Ok(symbol_index)
    }

    #[tokio::test]
    async fn test_basic_import_completion() -> Result<()> {
        let symbol_index = setup_test_workspace().await?;
        
        // Create import completion engine
        let import_manager = Arc::new(ImportManager::new()?);
        let symbol_index_arc = Arc::new(RwLock::new(Some(symbol_index)));
        let import_completion = ImportCompletionEngine::new(symbol_index_arc, import_manager);
        
        // Create completion context for typing "hel"
        let main_uri = Url::parse("file:///workspace/Main.gren")?;
        let context = CompletionContext {
            position: Position::new(5, 3),
            uri: main_uri.clone(),
            content: "module Main exposing (main)\n\nmain = hel".to_string(),
            trigger_character: None,
            line_prefix: "main = hel".to_string(),
            word_prefix: "hel".to_string(),
        };
        
        // Test import completion
        let import_items = import_completion.complete_unimported_symbols(&context).await?;
        
        // Should find "helper" function from Utils module
        assert!(!import_items.is_empty(), "Should find import completion items");
        
        let helper_items: Vec<_> = import_items.iter()
            .filter(|item| item.completion_item.label.contains("helper"))
            .collect();
        
        assert_eq!(helper_items.len(), 2, "Should have both exposed and qualified variants");
        
        // Check exposed variant
        let exposed_variant = helper_items.iter()
            .find(|item| item.completion_item.label == "helper")
            .expect("Should have exposed variant");
        
        assert!(exposed_variant.completion_item.detail.as_ref().unwrap().contains("import Utils exposing (helper)"));
        assert!(exposed_variant.completion_item.additional_text_edits.is_some());
        
        // Check qualified variant  
        let qualified_variant = helper_items.iter()
            .find(|item| item.completion_item.label == "Utils.helper")
            .expect("Should have qualified variant");
        
        assert!(qualified_variant.completion_item.detail.as_ref().unwrap().contains("import Utils"));
        assert!(qualified_variant.completion_item.additional_text_edits.is_some());
        
        println!("✅ Basic import completion test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_import_completion_performance() -> Result<()> {
        let symbol_index = setup_test_workspace().await?;
        
        // Create import completion engine
        let import_manager = Arc::new(ImportManager::new()?);
        let symbol_index_arc = Arc::new(RwLock::new(Some(symbol_index)));
        let import_completion = ImportCompletionEngine::new(symbol_index_arc, import_manager);
        
        // Create completion context
        let main_uri = Url::parse("file:///workspace/Main.gren")?;
        let context = CompletionContext {
            position: Position::new(5, 3),
            uri: main_uri.clone(),
            content: "module Main exposing (main)\n\nmain = hel".to_string(),
            trigger_character: None,
            line_prefix: "main = hel".to_string(),
            word_prefix: "hel".to_string(),
        };
        
        // Test completion performance
        let start = std::time::Instant::now();
        let _import_items = import_completion.complete_unimported_symbols(&context).await?;
        let elapsed = start.elapsed();
        
        // Should complete within performance requirements (< 150ms)
        assert!(elapsed.as_millis() < 150, "Import completion should be fast ({}ms)", elapsed.as_millis());
        
        println!("✅ Import completion performance test passed ({:?})", elapsed);
        Ok(())
    }

    #[tokio::test]
    async fn test_import_completion_with_existing_imports() -> Result<()> {
        let symbol_index = setup_test_workspace().await?;
        
        // Add existing imports to the symbol index  
        let main_uri = Url::parse("file:///workspace/Main.gren")?;
        let existing_tree_imports = vec![
            crate::tree_sitter_queries::ImportInfo {
                source_uri: main_uri.to_string(),
                imported_module: "Utils".to_string(),
                imported_symbols: Some(vec!["processor".to_string()]),
                alias_name: None,
                exposing_all: false,
            }
        ];
        
        symbol_index.update_imports_for_file(&main_uri, &existing_tree_imports).await?;
        
        // Create import completion engine
        let import_manager = Arc::new(ImportManager::new()?);
        let symbol_index_arc = Arc::new(RwLock::new(Some(symbol_index)));
        let import_completion = ImportCompletionEngine::new(symbol_index_arc, import_manager);
        
        // Test completion context for "hel" (should suggest helper, not processor)
        let context = CompletionContext {
            position: Position::new(8, 3),
            uri: main_uri.clone(),
            content: "module Main exposing (main)\n\nimport Utils exposing (processor)\n\nmain = \n    let\n        result = processor []\n        cleaned = hel".to_string(),
            trigger_character: None,
            line_prefix: "        cleaned = hel".to_string(),
            word_prefix: "hel".to_string(),
        };
        
        // Test import completion - should only suggest helper, not processor (already imported)
        let import_items = import_completion.complete_unimported_symbols(&context).await?;
        
        let helper_items: Vec<_> = import_items.iter()
            .filter(|item| item.completion_item.label.contains("helper"))
            .collect();
        
        let processor_items: Vec<_> = import_items.iter()
            .filter(|item| item.completion_item.label.contains("processor"))
            .collect();
        
        assert_eq!(helper_items.len(), 2, "Should suggest helper variants");
        assert_eq!(processor_items.len(), 0, "Should NOT suggest already imported processor");
        
        println!("✅ Import completion with existing imports test passed");
        Ok(())
    }
}