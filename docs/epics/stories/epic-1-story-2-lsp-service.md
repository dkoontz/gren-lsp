# Epic 1 Story 2: Core LSP Service Foundation

## 📋 User Story
**As a** Gren developer  
**I want** an LSP server that can initialize and communicate with my editor through tested LSP protocol messages  
**So that** I can establish a reliable foundation for language features with confidence in the communication layer

## ✅ Acceptance Criteria

### Core LSP Implementation
- [ ] Rust project initialized with proper dependencies (async-lsp, lsp-types, tokio, sqlx, tree-sitter)
- [ ] Basic LSP service trait implementation using async-lsp
- [ ] Initialize/initialized request handling with capability negotiation
- [ ] Shutdown/exit request handling  
- [ ] JSON-RPC communication over stdio working
- [ ] Server responds to LSP client with correct capabilities

### Integration Test Framework
- [ ] **LSP Test Harness**: Test framework that spawns fresh server process for each test
- [ ] **Stdio Communication**: Tests communicate with server over stdin/stdout using JSON-RPC
- [ ] **Timeout Handling**: All tests fail if no response received within 1000ms
- [ ] **Process Lifecycle**: Each test starts clean server process and disposes on completion
- [ ] **Message Validation**: Tests validate exact LSP message format and content

### Required Test Cases
- [ ] **Server Initialization Test**: Send `initialize` request, verify response, send `initialized` notification
- [ ] **Server Shutdown Test**: Send `shutdown` request, verify response, send `exit` notification, verify clean termination
- [ ] **Capability Negotiation Test**: Test multiple client capability combinations, verify appropriate server responses
- [ ] **Invalid Message Handling Test**: Send malformed JSON-RPC, verify error response, verify server stability
- [ ] **Message Ordering Test**: Test correct LSP lifecycle order, verify responses in expected sequence

## 🧪 Integration Test Requirements

### Test Framework Architecture
```
tests/
├── integration/
│   ├── lsp_lifecycle_tests.rs
│   ├── message_validation_tests.rs  
│   └── error_handling_tests.rs
├── helpers/
│   ├── lsp_test_client.rs
│   └── server_process.rs
└── fixtures/
    └── lsp_messages.json
```

### Test Implementation Pattern
```rust
#[tokio::test]
async fn test_server_initialization() {
    // Spawn fresh server process
    let mut server = LspTestServer::spawn().await;
    
    // Send initialize request
    let response = server.send_request_with_timeout(
        initialize_request(),
        Duration::from_millis(1000)
    ).await.expect("Server should respond to initialize");
    
    // Validate response structure and capabilities
    assert_initialize_response_valid(&response);
    
    // Clean shutdown
    server.shutdown().await;
}
```

### Test Scope Boundaries
- ✅ **In Scope**: LSP protocol message handling, server lifecycle, JSON-RPC communication
- ❌ **Out of Scope**: Editor client integration (reserved for Phase 5), language feature functionality

## ✅ Definition of Done
- LSP server starts and shuts down correctly
- Capability negotiation works programmatically (no manual client testing yet)
- All integration tests pass consistently
- No crashes during basic lifecycle operations
- Test suite runs in < 10 seconds total
- **100% test coverage** for implemented LSP message handlers

## 📁 Related Files
- `src/main.rs` (TO BE CREATED)
- `src/lsp_service.rs` (TO BE CREATED)
- `Cargo.toml` (TO BE CREATED)
- `tests/integration/` (TO BE CREATED)

## 🔗 Dependencies
- Epic 1 Story 1 completed (tree-sitter baseline)
- Rust toolchain 1.70+
- async-lsp, lsp-types, tokio crates

## 📊 Status
**Pending** - Awaiting Story 1 completion