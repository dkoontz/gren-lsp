# LSP Server Test Implementation Plan

## Testing Philosophy

### Core Principles
1. **Exact Input/Output Testing**: Every test must send actual LSP messages and assert on exact expected responses
2. **No Fallbacks**: No "to be implemented" placeholders, no fallback behaviors, no approximations
3. **Deterministic Results**: Each input has exactly one correct, deterministic output
4. **Complete Validation**: Tests must validate complete message structure, including all required fields, types, and values
5. **LSP Compliance**: All tests must follow LSP 3.18 specification exactly

### Test Structure
```rust
// Example test pattern
#[tokio::test]
async fn test_specific_lsp_feature() {
    // 1. Setup exact test state
    let mut server = create_test_server().await;
    
    // 2. Send exact LSP message
    let request = lsp_types::HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse("file:///test.gren").unwrap(),
            },
            position: Position { line: 5, character: 10 },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    
    // 3. Get response
    let response = server.hover(request).await.unwrap();
    
    // 4. Assert exact expected values
    assert_eq!(response.contents, HoverContents::Scalar(MarkedString::String(
        "greet : String -> String".to_string()
    )));
    assert_eq!(response.range, Some(Range {
        start: Position { line: 5, character: 0 },
        end: Position { line: 5, character: 5 },
    }));
}
```

---

## Phase 1: Fill Critical Gaps (High Priority)

### 1. Create `gren-lsp-core/tests/analysis_tests.rs`

#### Test: Analysis Workflow Coordination
**Purpose**: Test complete document analysis pipeline: parse → symbols → diagnostics

**Test Scenario**: `test_complete_analysis_workflow`
```rust
// Input: Document with function definition
let content = r#"
module Test exposing (greet)

greet : String -> String  
greet name =
    "Hello, " ++ name
"#;

// Expected: Complete analysis result
struct AnalysisResult {
    parse_errors: Vec<ParseError>,     // Expected: []
    symbols: Vec<Symbol>,              // Expected: [Function("greet")]
    diagnostics: Vec<Diagnostic>,      // Expected: []
}
```

**LSP Integration**: Test via `textDocument/didOpen` → analysis → `textDocument/publishDiagnostics`

#### Test: Analysis Result Caching
**Purpose**: Verify cached results returned when content unchanged

**Test Scenario**: `test_analysis_caching`
```rust
// 1. First analysis
let result1 = analyze_document(content).await;
let analysis_time1 = result1.analysis_duration;

// 2. Second analysis (same content)  
let result2 = analyze_document(content).await;
let analysis_time2 = result2.analysis_duration;

// Expected: Same results, second analysis much faster
assert_eq!(result1.symbols, result2.symbols);
assert!(analysis_time2 < analysis_time1 / 10); // Cache hit should be 10x+ faster
```

#### Test: Cross-File Analysis
**Purpose**: Test import resolution and dependency tracking

**Test Scenario**: `test_cross_file_import_resolution`
```rust
// File 1: Utils.gren
let utils_content = r#"
module Utils exposing (helper)

helper : String -> String
helper x = x ++ "!"  
"#;

// File 2: Main.gren  
let main_content = r#"
module Main exposing (main)

import Utils

main = Utils.helper "test"
"#;

// Expected: Import resolved, no errors
let main_analysis = analyze_with_dependencies(main_content, ["Utils.gren"]).await;
assert_eq!(main_analysis.diagnostics, vec![]); // No import errors
assert!(main_analysis.symbols.iter().any(|s| s.name == "Utils.helper")); // Import resolved
```

### 2. Create `gren-lsp-core/tests/document_tests.rs`

#### Test: Document Creation and Initialization
**Purpose**: Test conversion from LSP TextDocumentItem to internal Document

**Test Scenario**: `test_document_from_lsp_item`
```rust
// Input: LSP TextDocumentItem
let lsp_item = TextDocumentItem {
    uri: Url::parse("file:///test.gren").unwrap(),
    language_id: "gren".to_string(),
    version: 1,
    text: "module Test exposing (..)".to_string(),
};

// Create Document
let document = Document::from_lsp_item(lsp_item).unwrap();

// Expected: Exact field mapping
assert_eq!(document.uri().as_str(), "file:///test.gren");
assert_eq!(document.version(), 1);
assert_eq!(document.text(), "module Test exposing (..)");
assert_eq!(document.language_id(), "gren");
assert!(document.has_parse_tree()); // Should be parsed immediately
```

#### Test: Incremental Text Updates
**Purpose**: Test range-based content changes

**Test Scenario**: `test_incremental_text_update`
```rust
// Initial content
let initial = "module Test exposing (..)\n\ngreet = \"Hello\"";
let mut document = Document::new(uri, 1, initial);

// Change: Replace "Hello" with "Hi"  
let change = TextDocumentContentChangeEvent {
    range: Some(Range {
        start: Position { line: 2, character: 8 },
        end: Position { line: 2, character: 13 },
    }),
    range_length: Some(5),
    text: "Hi".to_string(),
};

// Apply change
document.apply_change(change, 2).unwrap();

// Expected: Exact updated content
assert_eq!(document.version(), 2);
assert_eq!(document.text(), "module Test exposing (..)\n\ngreet = \"Hi\"");
assert!(document.has_parse_tree()); // Should reparse incrementally
```

### 3. Create `gren-lsp-protocol/tests/handlers_tests.rs`

#### Test: textDocument/completion
**Purpose**: Test symbol completion with exact responses

**Test Scenario**: `test_completion_handler`
```rust
// Setup: Document with symbols
let content = r#"
module Test exposing (..)

greet : String -> String
greet name = "Hello, " ++ name

farewell : String -> String  
farewell name = "Goodbye, " ++ name
"#;

// LSP Request: Completion at position after "gr"
let request = CompletionParams {
    text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier {
            uri: Url::parse("file:///test.gren").unwrap(),
        },
        position: Position { line: 6, character: 2 }, // After "gr"
    },
    work_done_progress_params: WorkDoneProgressParams::default(),
    partial_result_params: PartialResultParams::default(),
    context: None,
};

// Expected Response: Exact completion items
let expected_response = CompletionResponse::Array(vec![
    CompletionItem {
        label: "greet".to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some("String -> String".to_string()),
        documentation: None,
        insert_text: Some("greet".to_string()),
        ..Default::default()
    }
]);

// Execute and assert
let response = completion_handler(request).await.unwrap();
assert_eq!(response, expected_response);
```

#### Test: textDocument/hover
**Purpose**: Test hover information with exact type display

**Test Scenario**: `test_hover_handler`
```rust
// Setup: Document with typed function
let content = r#"
module Test exposing (greet)

{-| Greets a person by name -}
greet : String -> String
greet name = "Hello, " ++ name
"#;

// LSP Request: Hover over "greet" function name
let request = HoverParams {
    text_document_position_params: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier {
            uri: Url::parse("file:///test.gren").unwrap(),
        },
        position: Position { line: 4, character: 2 }, // On "greet"
    },
    work_done_progress_params: WorkDoneProgressParams::default(),
};

// Expected Response: Exact hover content
let expected_response = Some(Hover {
    contents: HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: "```gren\ngreet : String -> String\n```\n\nGreets a person by name".to_string(),
    }),
    range: Some(Range {
        start: Position { line: 4, character: 0 },
        end: Position { line: 4, character: 5 },
    }),
});

// Execute and assert  
let response = hover_handler(request).await.unwrap();
assert_eq!(response, expected_response);
```

#### Test: textDocument/definition
**Purpose**: Test go-to-definition with exact location

**Test Scenario**: `test_definition_handler`
```rust
// Setup: Multi-line document with function call
let content = r#"
module Test exposing (..)

helper : String -> String
helper x = x ++ "!"

main = helper "test"
"#;

// LSP Request: Definition of "helper" in function call
let request = GotoDefinitionParams {
    text_document_position_params: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier {
            uri: Url::parse("file:///test.gren").unwrap(),
        },
        position: Position { line: 6, character: 7 }, // On "helper" call
    },
    work_done_progress_params: WorkDoneProgressParams::default(),
    partial_result_params: PartialResultParams::default(),
};

// Expected Response: Exact definition location
let expected_response = GotoDefinitionResponse::Scalar(Location {
    uri: Url::parse("file:///test.gren").unwrap(),
    range: Range {
        start: Position { line: 3, character: 0 },
        end: Position { line: 3, character: 6 }, // "helper" function name
    },
});

// Execute and assert
let response = definition_handler(request).await.unwrap();
assert_eq!(response, expected_response);
```

### 4. Create `gren-lsp-server/tests/server_tests.rs`

#### Test: LSP Initialization 
**Purpose**: Test complete initialize flow with exact capabilities

**Test Scenario**: `test_lsp_initialization`
```rust
// LSP Request: Initialize
let request = InitializeParams {
    process_id: Some(1234),
    root_path: None,
    root_uri: Some(Url::parse("file:///workspace").unwrap()),
    initialization_options: None,
    capabilities: ClientCapabilities {
        text_document: Some(TextDocumentClientCapabilities {
            completion: Some(CompletionClientCapabilities::default()),
            hover: Some(HoverClientCapabilities::default()),
            ..Default::default()
        }),
        ..Default::default()
    },
    trace: Some(TraceValue::Off),
    workspace_folders: None,
    client_info: Some(ClientInfo {
        name: "test-client".to_string(),
        version: Some("1.0.0".to_string()),
    }),
    locale: None,
};

// Expected Response: Exact server capabilities
let expected_response = InitializeResult {
    capabilities: ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL
        )),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    },
    server_info: Some(ServerInfo {
        name: "gren-lsp".to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }),
};

// Execute and assert
let response = server.initialize(request).await.unwrap();
assert_eq!(response, expected_response);
```

#### Test: Document Lifecycle
**Purpose**: Test didOpen → didChange → didClose flow

**Test Scenario**: `test_document_lifecycle`
```rust
// 1. didOpen notification
let did_open = DidOpenTextDocumentParams {
    text_document: TextDocumentItem {
        uri: Url::parse("file:///test.gren").unwrap(),
        language_id: "gren".to_string(),
        version: 1,
        text: "module Test exposing (..)".to_string(),
    },
};

server.did_open(did_open).await.unwrap();

// Verify: Document is tracked
assert!(server.workspace().is_document_open(&Url::parse("file:///test.gren").unwrap()));

// 2. didChange notification  
let did_change = DidChangeTextDocumentParams {
    text_document: VersionedTextDocumentIdentifier {
        uri: Url::parse("file:///test.gren").unwrap(),
        version: 2,
    },
    content_changes: vec![TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "module UpdatedTest exposing (..)".to_string(),
    }],
};

server.did_change(did_change).await.unwrap();

// Verify: Document content updated
let document = server.workspace().get_document(&Url::parse("file:///test.gren").unwrap()).unwrap();
assert_eq!(document.version(), 2);
assert!(document.text().contains("UpdatedTest"));

// 3. didClose notification
let did_close = DidCloseTextDocumentParams {
    text_document: TextDocumentIdentifier {
        uri: Url::parse("file:///test.gren").unwrap(),
    },
};

server.did_close(did_close).await.unwrap();

// Verify: Document no longer tracked (but symbols persist)
assert!(!server.workspace().is_document_open(&Url::parse("file:///test.gren").unwrap()));
```

---

## Phase 2: Edge Case Coverage (Medium Priority)

### 5. Error Handling Tests

#### Test: Malformed LSP Messages
**Purpose**: Test server response to invalid JSON and missing fields

**Test Scenarios**:
- `test_invalid_json_request`: Send malformed JSON → expect ParseError
- `test_missing_required_fields`: Send request without required fields → expect InvalidRequest
- `test_unknown_method`: Send request with unknown method → expect MethodNotFound

**Example**: `test_invalid_completion_request`
```rust
// Invalid request: missing position
let invalid_request = json!({
    "textDocument": {
        "uri": "file:///test.gren"
    }
    // Missing "position" field
});

// Expected: Exact error response
let expected_error = ResponseError {
    code: ErrorCode::InvalidRequest as i32,
    message: "Missing required field: position".to_string(),
    data: None,
};

let response = server.handle_raw_message(invalid_request).await;
assert_eq!(response.error, Some(expected_error));
```

#### Test: File System Errors
**Purpose**: Test handling of missing files, permission errors

**Test Scenarios**:
- `test_open_nonexistent_file`: Open file that doesn't exist → graceful handling
- `test_compiler_not_found`: Gren compiler not available → specific error message
- `test_workspace_permission_denied`: No read access to workspace → proper error

### 6. Concurrency Tests

#### Test: Concurrent Document Updates
**Purpose**: Test multiple didChange notifications on same document

**Test Scenario**: `test_concurrent_document_changes`  
```rust
// Send multiple changes simultaneously
let change1 = create_did_change("file:///test.gren", 2, "Version 2");
let change2 = create_did_change("file:///test.gren", 3, "Version 3");  
let change3 = create_did_change("file:///test.gren", 4, "Version 4");

// Execute concurrently
let results = tokio::join!(
    server.did_change(change1),
    server.did_change(change2), 
    server.did_change(change3)
);

// Expected: All succeed, final version is deterministic
assert!(results.0.is_ok());
assert!(results.1.is_ok());
assert!(results.2.is_ok());

let final_document = server.workspace().get_document(&uri).unwrap();
assert_eq!(final_document.version(), 4); // Latest version wins
assert!(final_document.text().contains("Version 4"));
```

---

## Phase 3: Integration & End-to-End (Medium Priority)

### 9. Full Integration Tests

#### Test: Complete LSP Workflow
**Purpose**: Test end-to-end LSP usage: open → navigate → edit → save → close

**Test Scenario**: `test_complete_lsp_workflow`
```rust
// 1. Initialize server
let init_response = server.initialize(create_init_params()).await.unwrap();
assert_eq!(init_response.capabilities.hover_provider, Some(HoverProviderCapability::Simple(true)));

// 2. Open document  
server.did_open(create_did_open("file:///main.gren", gren_content)).await.unwrap();

// 3. Test hover
let hover_response = server.hover(create_hover_params(5, 10)).await.unwrap();
assert_eq!(hover_response.unwrap().contents, expected_hover_content);

// 4. Test completion
let completion_response = server.completion(create_completion_params(10, 5)).await.unwrap();
assert_eq!(completion_response.unwrap(), expected_completions);

// 5. Edit document
server.did_change(create_did_change(2, "updated content")).await.unwrap();

// 6. Test definition after edit
let definition_response = server.definition(create_definition_params(8, 12)).await.unwrap();
assert_eq!(definition_response.unwrap(), expected_definition_location);

// 7. Close document
server.did_close(create_did_close("file:///main.gren")).await.unwrap();

// Expected: All operations succeed with exact responses
```

#### Test: Multi-File Project
**Purpose**: Test import resolution across multiple files

**Test Scenario**: `test_multi_file_project_navigation`
```rust
// Setup: Project with Utils.gren and Main.gren
let utils_content = create_utils_module();
let main_content = create_main_module_importing_utils();

// Open both files
server.did_open(create_did_open("file:///Utils.gren", utils_content)).await.unwrap();
server.did_open(create_did_open("file:///Main.gren", main_content)).await.unwrap();

// Test: Go to definition from Main.gren to Utils.gren
let definition_request = create_definition_params("file:///Main.gren", 5, 15); // On Utils.helper
let definition_response = server.definition(definition_request).await.unwrap();

// Expected: Jump to Utils.gren
let expected_location = Location {
    uri: Url::parse("file:///Utils.gren").unwrap(),
    range: Range {
        start: Position { line: 3, character: 0 },
        end: Position { line: 3, character: 6 },
    },
};
assert_eq!(definition_response.unwrap(), GotoDefinitionResponse::Scalar(expected_location));
```

### 11. Cross-Platform Tests

#### Test: Path Handling
**Purpose**: Test Windows vs Unix path separators and file URIs

**Test Scenarios**:
- `test_windows_file_paths`: Test `file:///C:/workspace/file.gren` URIs
- `test_unix_file_paths`: Test `file:///home/user/workspace/file.gren` URIs  
- `test_path_normalization`: Test path normalization across platforms

---

## Test Infrastructure Requirements

### Mock LSP Client
```rust
struct MockLspClient {
    server: LspServer,
    message_id: AtomicU64,
}

impl MockLspClient {
    async fn send_request<T: Request>(&self, params: T::Params) -> Result<T::Result> {
        // Send actual LSP message to server
        // Return exact response for validation
    }
    
    async fn send_notification<T: Notification>(&self, params: T::Params) -> Result<()> {
        // Send notification, ensure server processes it
    }
}
```

### Response Validation
```rust
fn assert_lsp_response_exact<T: PartialEq + Debug>(actual: T, expected: T) {
    assert_eq!(actual, expected, "LSP response must match exactly");
}

fn assert_diagnostic_exact(actual: &Diagnostic, expected: &Diagnostic) {
    assert_eq!(actual.range, expected.range);
    assert_eq!(actual.severity, expected.severity);  
    assert_eq!(actual.message, expected.message);
    assert_eq!(actual.source, expected.source);
    // Validate every field
}
```

### Test Fixtures
```rust
fn create_test_gren_project() -> TestProject {
    TestProject {
        files: hashmap! {
            "Main.gren" => include_str!("fixtures/Main.gren"),
            "Utils.gren" => include_str!("fixtures/Utils.gren"),
        },
        expected_symbols: load_expected_symbols(),
        expected_diagnostics: load_expected_diagnostics(),
    }
}
```

---

## Implementation Order

### Phase 1 Priority
1. `document_tests.rs` - Foundation for document management
2. `handlers_tests.rs` - Core LSP protocol functionality  
3. `server_tests.rs` - Server orchestration and lifecycle
4. `analysis_tests.rs` - Analysis pipeline validation

### Success Criteria
- Every test sends real LSP messages and validates exact responses
- No test has fallback logic or approximations
- All tests pass deterministically on every run
- Test coverage includes all major LSP features with precise validation
- Tests serve as executable specification for LSP behavior

This plan ensures comprehensive, exact testing of the LSP server with no ambiguity about expected behavior.