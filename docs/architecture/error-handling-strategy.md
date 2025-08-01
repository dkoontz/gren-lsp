# Error Handling Strategy

## LSP Protocol Errors
```rust
use lsp_types::{error_codes, ResponseError};

pub enum LspError {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerNotInitialized,
    UnknownErrorCode,
    ContentModified,
}

impl From<LspError> for ResponseError {
    fn from(error: LspError) -> Self {
        match error {
            LspError::ParseError => ResponseError {
                code: error_codes::PARSE_ERROR,
                message: "Parse error".to_string(),
                data: None,
            },
            LspError::InvalidRequest => ResponseError {
                code: error_codes::INVALID_REQUEST,
                message: "Invalid request".to_string(),
                data: None,
            },
            LspError::MethodNotFound => ResponseError {
                code: error_codes::METHOD_NOT_FOUND,
                message: "Method not found".to_string(),
                data: None,
            },
            LspError::ServerNotInitialized => ResponseError {
                code: error_codes::SERVER_NOT_INITIALIZED,
                message: "Server not initialized".to_string(),
                data: None,
            },
            // ... other error mappings
        }
    }
}
```

## Recovery Mechanisms
- **Partial Results**: Return available information when possible
- **Graceful Degradation**: Disable features on errors, not entire server
- **State Recovery**: Rebuild state from documents on corruption
- **Client Communication**: Inform client of capability changes

## Logging and Monitoring

### Logging Levels and Guidelines
The LSP server uses structured logging with four distinct levels:

**DEBUG Level** - Point-by-point diagnostic information:
```rust
log::debug!("Parsing tree-sitter node: {:?} at position {}:{}", node_kind, line, col);
log::debug!("  Found identifier: {}", identifier_name);
log::debug!("  Node children: {}", child_count);
log::debug!("Symbol '{}' found in scope at depth {}", symbol_name, scope_depth);
log::debug!("  Scope type: {}", scope_type);
log::debug!("  Parent scope: {}", parent_scope);
log::debug!("Test assertion: expected completion count {}, got {}", expected, actual);
log::debug!("  Item 0: {}", item_labels[0]);
log::debug!("  Item 1: {}", item_labels[1]);
log::debug!("Cache hit for document URI: {}", uri);
```

**INFO Level** - Main system checkpoints and LSP protocol activity:
```rust
log::info!("📨 LSP message received: {}", method_name);
log::info!("  Request ID: {}", request_id);
log::info!("  Parameters: {} bytes", param_size);
log::info!("📤 LSP response sent: {} ({}ms)", method_name, duration);
log::info!("  Response size: {} bytes", response_size);
log::info!("✅ Server initialized with capabilities: {:?}", capabilities);
log::info!("  Text sync: {}", sync_kind);
log::info!("  Completion triggers: {:?}", trigger_chars);
log::info!("📄 Document opened: {}", uri);
log::info!("  Language: {}", language_id);
log::info!("  Version: {}", version);
log::info!("🔨 Compilation started for project: {}", project_root);
log::info!("  Compiler path: {}", compiler_path);
log::info!("  Files to compile: {}", file_count);
log::info!("🧪 Test suite started: {}", test_name);
log::info!("  Test files: {}", test_file_count);
log::info!("  Setup helpers: {:?}", setup_helpers);
```

**WARNING Level** - Recoverable issues with alternatives:
```rust
log::warn!("⚠️  Parse tree incomplete, using cached version for {}", uri);
log::warn!("  Missing nodes: {}", missing_node_count);
log::warn!("  Cache age: {}ms", cache_age);
log::warn!("⚠️  Compiler took {}ms (>5000ms threshold), results may be stale", duration);
log::warn!("  File size: {} KB", file_size);
log::warn!("  Memory usage: {} MB", memory_usage);
log::warn!("⚠️  Document version mismatch: client={}, server={}", client_version, server_version);
log::warn!("  Applying version reconciliation");
log::warn!("  Content length: {} chars", content_length);
```

**ERROR Level** - Non-recoverable failures:
```rust
log::error!("❌ Gren compiler not found at path: {}", compiler_path);
log::error!("  Checked paths: {:?}", attempted_paths);
log::error!("  Working directory: {}", current_dir);
log::error!("❌ GREN_COMPILER_PATH environment variable not set");
log::error!("  Available env vars: {:?}", available_env_keys);
log::error!("  Process ID: {}", process_id);
log::error!("❌ Failed to write temporary file: {}", io_error);
log::error!("  Target path: {}", temp_file_path);
log::error!("  Disk space: {} MB", available_space);
log::error!("❌ SQLite database corruption detected: {}", db_error);
log::error!("  Database path: {}", db_path);
log::error!("  Last backup: {}", backup_timestamp);
```

### Logging Indentation Hierarchy
For related log messages that show process hierarchy:
```rust
log::info!("📄 Document opened: {}", uri);
log::info!("  Language: {}", language_id);
log::info!("  Version: {}", version);
log::debug!("    Parsing tree-sitter node: {:?} at position {}:{}", node_kind, line, col);
log::debug!("      Found identifier: {}", identifier_name);
log::debug!("      Node children: {}", child_count);
log::info!("📤 LSP response sent: {} ({}ms)", method_name, duration);
```

**Indentation Rules**:
- **2 spaces**: Additional information within the same logical step
- **4+ spaces**: Child steps or sub-processes at deeper levels
- **Emoji messages**: Always start at column 0 (no indentation)

### Logging Style Guidelines
- **Emoji Usage**: Only for "grouping" messages (📨 for incoming, 📤 for outgoing, ✅ for success, ❌ for errors, ⚠️ for warnings)
- **No Emoji in Details**: Individual assertions, sub-steps, and diagnostic details use plain text that is indented to show membership in higher level log
- **Structured Context**: Include request IDs, timing, and relevant identifiers
- **Consistent Formatting**: Use consistent patterns for similar operations

### Performance Metrics and Error Tracking
- **Request Timing**: Track response times with request correlation IDs
- **Resource Usage**: Monitor memory usage and compilation times
- **Error Context**: Detailed error information for debugging without sensitive data
- **Health Monitoring**: Server status and capability reporting
