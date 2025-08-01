# Testing Architecture

For complete test specifications with JSON-RPC message sequences, see [lsp/test-cases.md](../lsp/test-cases.md).

## Unit Testing
- **Component Isolation**: Mock dependencies for unit tests using lsp-types structures
- **Handler Testing**: Test LSP handlers with type-safe sample requests
- **Parser Testing**: Validate tree-sitter queries and parsing
- **Compiler Interface**: Mock compiler responses using lsp-types::Diagnostic

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::*;

    #[tokio::test]
    async fn test_completion_handler() {
        let server = create_test_server().await;

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse("file:///test.gren").unwrap(),
                },
                position: Position { line: 5, character: 10 },
            },
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = server.completion(params).await.unwrap();
        assert!(result.is_some());
    }
}

## Integration Testing
- **File-Based Test Sequences**: All integration tests use static JSON files on disk for easy inspection and debugging
- **LSP Protocol**: Full JSON-RPC message testing with lsp-types serialization using predefined test files
- **Document Lifecycle**: Test document synchronization flows using lsp-textdocument with isolated test scenarios
- **Feature Integration**: End-to-end feature testing with type-safe message construction from test case files
- **Error Scenarios**: Test error handling using lsp-types::ResponseError with comprehensive error test files
- **Position Mapping**: Test UTF-16 position calculations with lsp-textdocument using position-specific test cases

```rust
#[tokio::test]
async fn test_document_lifecycle_with_positions() {
    let mut server = create_test_server().await;

    // Test didOpen
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::parse("file:///test.gren").unwrap(),
            language_id: "gren".to_string(),
            version: 1,
            text: "module Main exposing (main)".to_string(),
        },
    };
    server.did_open(open_params).await;

    // Test incremental change using lsp-textdocument
    let change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::parse("file:///test.gren").unwrap(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 26 },
                end: Position { line: 0, character: 26 },
            }),
            range_length: None,
            text: "\n\nmain = 42".to_string(),
        }],
    };
    server.did_change(change_params).await;

    // Test position mapping
    let uri = Url::parse("file:///test.gren").unwrap();
    let position = server.get_position_from_offset(&uri, 35).unwrap();
    assert_eq!(position.line, 2);
    assert_eq!(position.character, 8);

    // Verify incremental parsing worked
    assert!(server.get_parse_tree(&uri).is_some());
}

## Performance Testing
- **Load Testing**: Handle multiple concurrent requests using test file scenarios
- **Memory Testing**: Monitor memory usage under load with repeated test execution
- **Latency Testing**: Measure response times using test case benchmarks
- **Scalability Testing**: Test with large projects using multi-document test scenarios

## Test File Organization
The testing approach uses static JSON files organized in `lsp/tests/` with:
- **helpers/**: Reusable setup and cleanup sequences
- **lifecycle/**: Server initialization and shutdown tests
- **completion/**: Code completion test scenarios
- **hover/**: Hover information test cases
- **definition/**: Go-to-definition test sequences
- **references/**: Find references test scenarios
- **diagnostics/**: Error reporting test cases

Each test file specifies required setup helpers and expected message sequences, ensuring complete test isolation and repeatability.

## Test Assertion and Failure Reporting
The testing framework must provide clear, actionable failure messages:

```rust
// Use a testing framework that provides detailed diff output
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq; // Provides colored diff output

    #[test]
    fn test_completion_response() {
        let expected_response = CompletionResponse::Array(vec![
            CompletionItem {
                label: "text".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("String -> Html msg".to_string()),
                ..Default::default()
            }
        ]);

        let actual_response = server.handle_completion(params).await.unwrap();

        // This will show a detailed diff if the test fails:
        //
        // thread 'test_completion_response' panicked at:
        // assertion failed: `(left == right)`
        //   left: `CompletionItem { label: "text", kind: Function, detail: "String -> Html msg" }`
        //  right: `CompletionItem { label: "text", kind: Function, detail: "String -> Html Msg" }`
        //                                                                              ^^^
        assert_eq!(expected_response, actual_response);
    }
}
```

**Test Failure Requirements**:
- **Clear Expected vs Actual**: Show exactly what was expected and what was received
- **Diff Visualization**: Use a testing framework with built-in diff support (e.g., `pretty_assertions`, `insta`)
- **No Custom Diff Implementation**: Rely on existing Rust testing ecosystem rather than building custom diff logic
- **Precise Error Messages**: Highlight the exact field or value that differs
