//! Integration tests for code actions functionality
//! 
//! Tests textDocument/codeAction LSP method including:
//! - Missing import suggestions
//! - Type signature additions  
//! - Unused import removal
//! - LSP protocol compliance

#[cfg(test)]
mod tests {
    use crate::code_actions::CodeActionsEngine;
    use crate::symbol_index::SymbolIndex;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tower_lsp::lsp_types::*;
    use url::Url;

    /// Create a test code actions engine with an in-memory symbol index
    async fn create_test_engine() -> CodeActionsEngine {
        let workspace_root = std::env::current_dir().unwrap();
        let symbol_index = SymbolIndex::new_in_memory(workspace_root).await.unwrap();
        
        // Add some test symbols to the index
        add_test_symbols(&symbol_index).await;
        
        CodeActionsEngine::new(Arc::new(RwLock::new(Some(symbol_index)))).unwrap()
    }

    /// Add test symbols to the symbol index for testing import suggestions
    async fn add_test_symbols(symbol_index: &SymbolIndex) {
        use crate::symbol_index::Symbol;
        
        let test_uri = Url::parse("file:///test/Http.gren").unwrap();
        let test_range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 10 },
        };
        
        // Add Http.get function
        let http_get_symbol = Symbol::new(
            "get".to_string(),
            SymbolKind::FUNCTION,
            &test_uri,
            test_range,
            Some("Http".to_string()),
            Some("get : String -> Task Http.Error String".to_string()),
            None,
        );
        
        // Add Html.text function
        let html_text_symbol = Symbol::new(
            "text".to_string(),
            SymbolKind::FUNCTION,
            &test_uri,
            test_range,
            Some("Html".to_string()),
            Some("text : String -> Html msg".to_string()),
            None,
        );
        
        symbol_index.add_symbol(&http_get_symbol).await.unwrap();
        symbol_index.add_symbol(&html_text_symbol).await.unwrap();
    }

    /// Test basic code action response structure
    #[tokio::test]
    async fn test_code_action_response_structure() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri },
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        let document_content = "main = greet \"World\"";
        
        let result = engine.handle_code_action(params, document_content).await;
        assert!(result.is_ok());
    }

    /// Test missing import code action generation
    #[tokio::test]
    async fn test_missing_import_code_action() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        
        // Create a diagnostic for undefined 'get' symbol
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 8 },
                end: Position { line: 0, character: 11 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "I cannot find 'get'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };
        
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri },
            range: diagnostic.range,
            context: CodeActionContext {
                diagnostics: vec![diagnostic],
                only: Some(vec![CodeActionKind::QUICKFIX]),
                trigger_kind: Some(CodeActionTriggerKind::AUTOMATIC),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        let document_content = "main = get \"https://example.com\"";
        
        let result = engine.handle_code_action(params, document_content).await.unwrap();
        
        // Should return at least one code action for import suggestion
        assert!(result.is_some());
        let actions = result.unwrap();
        assert!(!actions.is_empty());
        
        // Check that we got a quickfix action
        let first_action = &actions[0];
        if let CodeActionOrCommand::CodeAction(code_action) = first_action {
            assert_eq!(code_action.kind, Some(CodeActionKind::QUICKFIX));
            assert!(code_action.title.contains("Import"));
            assert!(code_action.title.contains("get"));
            assert!(code_action.edit.is_some());
        } else {
            panic!("Expected CodeAction, got Command");
        }
    }

    /// Test add type signature code action
    #[tokio::test]
    async fn test_add_type_signature_code_action() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri },
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: Some(vec![CodeActionKind::REFACTOR_REWRITE]),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        // Function without type signature
        let document_content = "greet name = \"Hello, \" ++ name";
        
        let result = engine.handle_code_action(params, document_content).await.unwrap();
        
        // Should return code action for adding type signature
        // Since we have no diagnostics, cursor-based actions should be generated
        if let Some(actions) = result {
            let refactor_actions: Vec<_> = actions
                .iter()
                .filter(|action| {
                    if let CodeActionOrCommand::CodeAction(code_action) = action {
                        code_action.kind == Some(CodeActionKind::REFACTOR_REWRITE)
                    } else {
                        false
                    }
                })
                .collect();
            
            // We should find at least one refactor action for adding type signature
            // The tree-sitter queries should detect the `greet` function
            assert!(!refactor_actions.is_empty(), "Expected at least one refactor action for function without type signature");
            
            // Verify the first action is for adding a type signature
            if let CodeActionOrCommand::CodeAction(code_action) = &refactor_actions[0] {
                assert!(code_action.title.contains("type signature"), "Expected type signature action, got: {}", code_action.title);
                assert!(code_action.title.contains("greet"), "Expected action for 'greet' function, got: {}", code_action.title);
            }
        } else {
            panic!("Expected some code actions for function without type signature, but got None");
        }
    }

    /// Test code action filtering by kind
    #[tokio::test] 
    async fn test_code_action_filtering_by_kind() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        
        // Create diagnostics that could generate multiple types of actions
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 8 },
                end: Position { line: 0, character: 11 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "I cannot find 'text'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };
        
        // Test filtering for only quickfix actions
        let quickfix_params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri.clone() },
            range: diagnostic.range,
            context: CodeActionContext {
                diagnostics: vec![diagnostic.clone()],
                only: Some(vec![CodeActionKind::QUICKFIX]),
                trigger_kind: Some(CodeActionTriggerKind::AUTOMATIC),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        let document_content = "main = text \"Hello\"";
        
        let quickfix_result = engine.handle_code_action(quickfix_params, document_content).await.unwrap();
        
        if let Some(actions) = quickfix_result {
            // All actions should be quickfix
            for action in &actions {
                if let CodeActionOrCommand::CodeAction(code_action) = action {
                    assert_eq!(code_action.kind, Some(CodeActionKind::QUICKFIX));
                }
            }
        }
        
        // Test filtering for refactor actions only
        let refactor_params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri },
            range: diagnostic.range,
            context: CodeActionContext {
                diagnostics: vec![diagnostic],
                only: Some(vec![CodeActionKind::REFACTOR]),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        let refactor_result = engine.handle_code_action(refactor_params, document_content).await.unwrap();
        
        // Should return no actions since we don't have refactor actions for missing imports
        // Refactor actions are only cursor-based and since we have diagnostics, they shouldn't be triggered
        assert!(refactor_result.is_none() || refactor_result.as_ref().unwrap().is_empty(), 
                "Expected no refactor actions for missing import diagnostics, but got {:?}", refactor_result);
    }

    /// Test empty response for invalid symbol names
    #[tokio::test]
    async fn test_no_actions_for_unknown_symbols() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 8 },
                end: Position { line: 0, character: 20 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "I cannot find 'unknownFunction'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };
        
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri },
            range: diagnostic.range,
            context: CodeActionContext {
                diagnostics: vec![diagnostic],
                only: None,
                trigger_kind: Some(CodeActionTriggerKind::AUTOMATIC),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        let document_content = "main = unknownFunction ()";
        
        let result = engine.handle_code_action(params, document_content).await.unwrap();
        
        // Should return empty or None for unknown symbols
        if let Some(actions) = result {
            assert!(actions.is_empty(), "Expected no actions but got {} actions", actions.len());
        }
    }

    /// Test WorkspaceEdit structure in code actions
    #[tokio::test]
    async fn test_workspace_edit_structure() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 8 },
                end: Position { line: 0, character: 12 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "I cannot find 'text'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };
        
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri.clone() },
            range: diagnostic.range,
            context: CodeActionContext {
                diagnostics: vec![diagnostic],
                only: Some(vec![CodeActionKind::QUICKFIX]),
                trigger_kind: Some(CodeActionTriggerKind::AUTOMATIC),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        let document_content = "main = text \"Hello\"";
        
        let result = engine.handle_code_action(params, document_content).await.unwrap();
        
        if let Some(actions) = result {
            if !actions.is_empty() {
                if let CodeActionOrCommand::CodeAction(code_action) = &actions[0] {
                    assert!(code_action.edit.is_some());
                    
                    let edit = code_action.edit.as_ref().unwrap();
                    assert!(edit.changes.is_some());
                    
                    let changes = edit.changes.as_ref().unwrap();
                    assert!(changes.contains_key(&test_uri));
                    
                    let file_edits = &changes[&test_uri];
                    assert!(!file_edits.is_empty());
                    
                    // Verify the edit adds an import statement
                    let text_edit = &file_edits[0];
                    assert!(text_edit.new_text.contains("import"));
                    assert!(text_edit.new_text.contains("text"));
                }
            }
        }
    }

    /// Test LSP protocol compliance
    #[tokio::test]
    async fn test_lsp_protocol_compliance() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri },
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        let document_content = "main = greet \"World\"";
        
        let result = engine.handle_code_action(params, document_content).await;
        
        // Should not error, even with no matching actions
        assert!(result.is_ok());
        
        if let Ok(Some(actions)) = result {
            for action in actions {
                match action {
                    CodeActionOrCommand::CodeAction(code_action) => {
                        // Verify required fields are present
                        assert!(!code_action.title.is_empty());
                        
                        // Verify at least one of edit or command is present
                        assert!(code_action.edit.is_some() || code_action.command.is_some());
                        
                        // Verify kind is valid
                        if let Some(ref kind) = code_action.kind {
                            assert!(!kind.as_str().is_empty());
                        }
                    }
                    CodeActionOrCommand::Command(command) => {
                        // Verify command structure
                        assert!(!command.title.is_empty());
                        assert!(!command.command.is_empty());
                    }
                }
            }
        }
    }

    /// Test error handling for invalid tree-sitter parsing
    #[tokio::test]
    async fn test_error_handling_invalid_syntax() {
        let mut engine = create_test_engine().await;
        
        let test_uri = Url::parse("file:///test.gren").unwrap();
        
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: test_uri },
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        
        // Invalid Gren syntax
        let document_content = "this is not valid gren syntax !!!";
        
        let result = engine.handle_code_action(params, document_content).await;
        
        // Should handle invalid syntax gracefully
        assert!(result.is_ok());
    }
}