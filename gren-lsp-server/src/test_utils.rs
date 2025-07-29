use lsp_types::*;

/// Test utilities for LSP server testing
pub struct LspTestClient {}

impl LspTestClient {
    pub fn new() -> Self {
        Self {}
    }

    /// Create an initialize request with basic capabilities
    pub fn create_initialize_request() -> InitializeParams {
        InitializeParams {
            process_id: Some(1234),
            root_path: Some("/test/workspace".to_string()),
            root_uri: Some(Url::parse("file:///test/workspace").unwrap()),
            initialization_options: None,
            capabilities: ClientCapabilities {
                workspace: Some(WorkspaceClientCapabilities {
                    apply_edit: Some(true),
                    workspace_edit: Some(WorkspaceEditClientCapabilities {
                        document_changes: Some(true),
                        resource_operations: Some(vec![
                            ResourceOperationKind::Create,
                            ResourceOperationKind::Rename,
                            ResourceOperationKind::Delete,
                        ]),
                        failure_handling: Some(FailureHandlingKind::Abort),
                        normalizes_line_endings: Some(true),
                        change_annotation_support: None,
                    }),
                    did_change_configuration: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    did_change_watched_files: Some(DidChangeWatchedFilesClientCapabilities {
                        dynamic_registration: Some(true),
                        relative_pattern_support: Some(true),
                    }),
                    symbol: Some(WorkspaceSymbolClientCapabilities {
                        dynamic_registration: Some(true),
                        symbol_kind: Some(SymbolKindCapability {
                            value_set: Some(vec![
                                SymbolKind::FILE,
                                SymbolKind::MODULE,
                                SymbolKind::NAMESPACE,
                                SymbolKind::FUNCTION,
                                SymbolKind::VARIABLE,
                                SymbolKind::CONSTANT,
                            ]),
                        }),
                        tag_support: None,
                        resolve_support: None,
                    }),
                    execute_command: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    workspace_folders: Some(true),
                    configuration: Some(true),
                    semantic_tokens: None,
                    code_lens: None,
                    file_operations: None,
                    inline_value: None,
                    inlay_hint: None,
                    diagnostic: None,
                }),
                text_document: Some(TextDocumentClientCapabilities {
                    synchronization: Some(TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(true),
                        will_save: Some(true),
                        will_save_wait_until: Some(true),
                        did_save: Some(true),
                    }),
                    completion: Some(CompletionClientCapabilities {
                        dynamic_registration: Some(true),
                        completion_item: Some(CompletionItemCapability {
                            snippet_support: Some(true),
                            commit_characters_support: Some(true),
                            documentation_format: Some(vec![
                                MarkupKind::Markdown,
                                MarkupKind::PlainText,
                            ]),
                            deprecated_support: Some(true),
                            preselect_support: Some(true),
                            tag_support: Some(TagSupport {
                                value_set: vec![CompletionItemTag::DEPRECATED],
                            }),
                            insert_replace_support: Some(true),
                            resolve_support: Some(CompletionItemCapabilityResolveSupport {
                                properties: vec!["documentation".to_string(), "detail".to_string()],
                            }),
                            insert_text_mode_support: None,
                            label_details_support: None,
                        }),
                        completion_item_kind: Some(CompletionItemKindCapability {
                            value_set: Some(vec![
                                CompletionItemKind::TEXT,
                                CompletionItemKind::METHOD,
                                CompletionItemKind::FUNCTION,
                                CompletionItemKind::VARIABLE,
                                CompletionItemKind::MODULE,
                                CompletionItemKind::KEYWORD,
                            ]),
                        }),
                        context_support: Some(true),
                        insert_text_mode: None,
                        completion_list: None,
                    }),
                    hover: Some(HoverClientCapabilities {
                        dynamic_registration: Some(true),
                        content_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                    }),
                    signature_help: Some(SignatureHelpClientCapabilities {
                        dynamic_registration: Some(true),
                        signature_information: Some(SignatureInformationSettings {
                            documentation_format: Some(vec![
                                MarkupKind::Markdown,
                                MarkupKind::PlainText,
                            ]),
                            parameter_information: Some(ParameterInformationSettings {
                                label_offset_support: Some(true),
                            }),
                            active_parameter_support: Some(true),
                        }),
                        context_support: Some(true),
                    }),
                    references: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    document_highlight: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    document_symbol: Some(DocumentSymbolClientCapabilities {
                        dynamic_registration: Some(true),
                        symbol_kind: Some(SymbolKindCapability {
                            value_set: Some(vec![
                                SymbolKind::FILE,
                                SymbolKind::MODULE,
                                SymbolKind::FUNCTION,
                                SymbolKind::VARIABLE,
                                SymbolKind::CONSTANT,
                            ]),
                        }),
                        hierarchical_document_symbol_support: Some(true),
                        tag_support: None,
                    }),
                    formatting: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    range_formatting: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    on_type_formatting: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    declaration: Some(GotoCapability {
                        dynamic_registration: Some(true),
                        link_support: Some(true),
                    }),
                    definition: Some(GotoCapability {
                        dynamic_registration: Some(true),
                        link_support: Some(true),
                    }),
                    type_definition: Some(GotoCapability {
                        dynamic_registration: Some(true),
                        link_support: Some(true),
                    }),
                    implementation: Some(GotoCapability {
                        dynamic_registration: Some(true),
                        link_support: Some(true),
                    }),
                    code_action: Some(CodeActionClientCapabilities {
                        dynamic_registration: Some(true),
                        code_action_literal_support: Some(CodeActionLiteralSupport {
                            code_action_kind: CodeActionKindLiteralSupport {
                                value_set: vec![
                                    "quickfix".to_string(),
                                    "refactor".to_string(),
                                    "source".to_string(),
                                ],
                            },
                        }),
                        is_preferred_support: Some(true),
                        disabled_support: Some(true),
                        data_support: Some(true),
                        resolve_support: None,
                        honors_change_annotations: None,
                    }),
                    code_lens: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    document_link: Some(DocumentLinkClientCapabilities {
                        dynamic_registration: Some(true),
                        tooltip_support: Some(true),
                    }),
                    color_provider: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    rename: Some(RenameClientCapabilities {
                        dynamic_registration: Some(true),
                        prepare_support: Some(true),
                        prepare_support_default_behavior: Some(
                            PrepareSupportDefaultBehavior::IDENTIFIER,
                        ),
                        honors_change_annotations: Some(true),
                    }),
                    publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                        related_information: Some(true),
                        tag_support: Some(TagSupport {
                            value_set: vec![DiagnosticTag::UNNECESSARY, DiagnosticTag::DEPRECATED],
                        }),
                        version_support: Some(true),
                        code_description_support: Some(true),
                        data_support: Some(true),
                    }),
                    folding_range: Some(FoldingRangeClientCapabilities {
                        dynamic_registration: Some(true),
                        range_limit: Some(5000),
                        line_folding_only: Some(false),
                        folding_range_kind: None,
                        folding_range: None,
                    }),
                    selection_range: Some(SelectionRangeClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    linked_editing_range: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    call_hierarchy: Some(CallHierarchyClientCapabilities {
                        dynamic_registration: Some(true),
                    }),
                    semantic_tokens: None,
                    moniker: None,
                    type_hierarchy: None,
                    inline_value: None,
                    inlay_hint: None,
                    diagnostic: None,
                }),
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(true),
                    show_message: None,
                    show_document: None,
                }),
                general: Some(GeneralClientCapabilities {
                    regular_expressions: Some(RegularExpressionsClientCapabilities {
                        engine: "ECMAScript".to_string(),
                        version: Some("ES2020".to_string()),
                    }),
                    markdown: Some(MarkdownClientCapabilities {
                        parser: "marked".to_string(),
                        version: Some("1.1.0".to_string()),
                        allowed_tags: None,
                    }),
                    stale_request_support: None,
                    position_encodings: None,
                }),
                experimental: None,
            },
            trace: Some(TraceValue::Verbose),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::parse("file:///test/workspace").unwrap(),
                name: "test-workspace".to_string(),
            }]),
            client_info: Some(ClientInfo {
                name: "test-client".to_string(),
                version: Some("1.0.0".to_string()),
            }),
            locale: None,
        }
    }

    /// Create a test document open notification
    pub fn create_did_open_notification(
        uri: &str,
        language_id: &str,
        content: &str,
    ) -> DidOpenTextDocumentParams {
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(uri).unwrap(),
                language_id: language_id.to_string(),
                version: 1,
                text: content.to_string(),
            },
        }
    }

    /// Create a test document change notification
    pub fn create_did_change_notification(
        uri: &str,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> DidChangeTextDocumentParams {
        DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: Url::parse(uri).unwrap(),
                version,
            },
            content_changes: changes,
        }
    }

    /// Create a completion request
    pub fn create_completion_request(uri: &str, line: u32, character: u32) -> CompletionParams {
        CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse(uri).unwrap(),
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        }
    }

    /// Create a go-to-definition request
    pub fn create_goto_definition_request(
        uri: &str,
        line: u32,
        character: u32,
    ) -> GotoDefinitionParams {
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse(uri).unwrap(),
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Create a hover request
    pub fn create_hover_request(uri: &str, line: u32, character: u32) -> HoverParams {
        HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse(uri).unwrap(),
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }
}

/// Helper function to create test Gren source code
pub fn create_test_gren_source() -> &'static str {
    r#"module TestModule exposing (..)

import Array

type alias User =
    { name : String
    , age : Int
    }

type Message
    = UpdateUser User
    | DeleteUser String

greetUser : User -> String
greetUser user =
    "Hello, " ++ user.name

processUsers : Array User -> Array String  
processUsers users =
    Array.map greetUser users
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_initialize_request() {
        let params = LspTestClient::create_initialize_request();
        assert!(params.capabilities.text_document.is_some());
        assert!(params.capabilities.workspace.is_some());
        assert_eq!(params.client_info.as_ref().unwrap().name, "test-client");
    }

    #[test]
    fn test_create_did_open_notification() {
        let params = LspTestClient::create_did_open_notification(
            "file:///test.gren",
            "gren",
            "module Test exposing (..)",
        );
        assert_eq!(params.text_document.uri.as_str(), "file:///test.gren");
        assert_eq!(params.text_document.language_id, "gren");
        assert_eq!(params.text_document.version, 1);
    }

    #[test]
    fn test_create_completion_request() {
        let params = LspTestClient::create_completion_request("file:///test.gren", 5, 10);
        assert_eq!(params.text_document_position.position.line, 5);
        assert_eq!(params.text_document_position.position.character, 10);
    }

    #[test]
    fn test_gren_source_creation() {
        let source = create_test_gren_source();
        assert!(source.contains("module TestModule"));
        assert!(source.contains("greetUser"));
        assert!(source.contains("User"));
    }
}
