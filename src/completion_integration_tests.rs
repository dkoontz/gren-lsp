#[cfg(test)]
mod completion_integration_tests {
    use crate::completion::CompletionEngine;
    use tempfile::TempDir;
    use std::time::Instant;
    use tower_lsp::lsp_types::*;
    use url::Url;
    use anyhow::Result;
    
    /// Test completion engine creation and basic functionality
    #[tokio::test]
    async fn test_completion_basic_workflow() -> Result<()> {
        // Create temporary workspace
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Use in-memory database for testing
        let symbol_index = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root).await?;
        
        // Initialize completion engine
        let completion_engine = CompletionEngine::new(symbol_index)?;
        
        // Test basic completion context building
        let uri = Url::parse("file:///test/Main.gren")?;
        let position = Position::new(2, 19); // At end of "myFunction input = "
        
        let text_document_position = TextDocumentPositionParams::new(
            TextDocumentIdentifier::new(uri.clone()),
            position,
        );
        
        let completion_params = CompletionParams {
            text_document_position: text_document_position,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(tower_lsp::lsp_types::CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        };
        
        // Test that completion engine responds without errors
        let document_content = "module Main exposing (..)\n\nmyFunction input = ";
        let start_time = Instant::now();
        
        // This should not fail even if it returns empty results
        let result = completion_engine.handle_completion(completion_params, document_content).await;
        let duration = start_time.elapsed();
        
        // Verify it responds quickly and doesn't error
        match result {
            Ok(_) => {
                assert!(duration.as_millis() < 100, "Should respond within 100ms");
            }
            Err(e) => {
                eprintln!("Completion error: {:?}", e);
                panic!("Completion should not error: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Test that we can create completion contexts correctly
    #[test]
    fn test_completion_context_creation() {
        let uri = Url::parse("file:///test/Main.gren").unwrap();
        let position = Position::new(5, 4);
        
        let text_document_position = TextDocumentPositionParams::new(
            TextDocumentIdentifier::new(uri.clone()),
            position,
        );
        
        let completion_params = CompletionParams {
            text_document_position: text_document_position,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(tower_lsp::lsp_types::CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        };
        
        // Verify completion params structure
        assert_eq!(completion_params.text_document_position.text_document.uri, uri);
        assert_eq!(completion_params.text_document_position.position, position);
        assert!(completion_params.context.is_some());
        
        if let Some(context) = completion_params.context {
            assert_eq!(context.trigger_kind, CompletionTriggerKind::TRIGGER_CHARACTER);
            assert_eq!(context.trigger_character, Some(".".to_string()));
        }
    }
    
    /// Test completion item structure
    #[test]
    fn test_completion_item_creation() {
        // Test creating different types of completion items
        let keyword_item = CompletionItem {
            label: "let".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Gren keyword".to_string()),
            documentation: Some(Documentation::String("Local variable binding".to_string())),
            insert_text: Some("let".to_string()),
            ..Default::default()
        };
        
        assert_eq!(keyword_item.label, "let");
        assert_eq!(keyword_item.kind, Some(CompletionItemKind::KEYWORD));
        assert!(keyword_item.detail.is_some());
        assert!(keyword_item.documentation.is_some());
        
        let function_item = CompletionItem {
            label: "myFunction".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("from Main".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "```gren\nmyFunction : String -> String\n```".to_string(),
            })),
            insert_text: Some("myFunction".to_string()),
            ..Default::default()
        };
        
        assert_eq!(function_item.label, "myFunction");
        assert_eq!(function_item.kind, Some(CompletionItemKind::FUNCTION));
        
        if let Some(Documentation::MarkupContent(content)) = function_item.documentation {
            assert_eq!(content.kind, MarkupKind::Markdown);
            assert!(content.value.contains("myFunction"));
        }
    }
    
    /// Test completion engine performance characteristics
    #[tokio::test]
    async fn test_completion_performance_characteristics() -> Result<()> {
        // Create temporary workspace
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        
        // Use in-memory database for testing
        let symbol_index = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root).await?;
        let _completion_engine = CompletionEngine::new(symbol_index)?;
        
        // Test that completion engine creation is fast
        let start = Instant::now();
        
        for _ in 0..10 {
            let temp_dir_inner = TempDir::new()?;
            let workspace_root_inner = temp_dir_inner.path().to_path_buf();
            let symbol_index_inner = crate::symbol_index::SymbolIndex::new_in_memory(workspace_root_inner).await?;
            let _completion_engine_inner = CompletionEngine::new(symbol_index_inner)?;
        }
        
        let duration = start.elapsed();
        assert!(duration.as_millis() < 100, "Completion engine creation should be fast");
        
        Ok(())
    }
}