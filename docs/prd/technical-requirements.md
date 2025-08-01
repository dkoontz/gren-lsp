# Technical Requirements

## Architecture Constraints

### LSP Framework
- Use async-lsp crate for asynchronous message handling
- Implement LspService trait for both server and potential client functionality
- Leverage tower middleware for common LSP features
- Handle notifications synchronously to maintain correct ordering

### Tree-sitter Integration
- All parsing must use tree-sitter, not regex or string matching
- Implement incremental parsing for performance
- Maintain parse trees for all open documents
- Use tree-sitter queries for symbol extraction

### Compiler Integration
- Invoke external Gren compiler specified by environment variable
- Write in-memory document states to temporary files for compilation
- Parse compiler output for diagnostic information
- Cache compilation results when possible

### Performance Requirements
- **Response Times**:
  - Completion: < 100ms for 95% of requests
  - Hover: < 50ms for 95% of requests
  - Go-to-definition: < 200ms for 95% of requests
  - Diagnostics: < 500ms after document change
- **Memory Usage**: < 100MB for typical projects (< 50 files)
- **Startup Time**: < 2 seconds for project initialization

### Reliability Requirements
- **Error Handling**: Never crash on malformed input
- **Graceful Degradation**: Provide partial results when possible
- **Accurate Results**: Prefer no result over incorrect result
- **State Consistency**: Maintain correct document state across all operations

## Data Management

### Document Storage
- Maintain in-memory copies of all open documents
- Apply incremental changes correctly
- Track document versions to prevent race conditions
- Implement LRU cache for closed documents (default 100 items)

### Symbol Indexing
- Use SQLite database for persistent symbol storage
- Index all project symbols at startup
- Incrementally update index on file changes
- Support cross-module symbol resolution

### Workspace Management
- Support single-folder and multi-folder workspaces
- Detect Gren project structure (gren.json, src/ directory)
- Handle workspace configuration changes
- Monitor file system changes for non-open files
