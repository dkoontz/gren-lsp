# Coding Standards

This document defines MANDATORY coding standards for AI agents and human developers working on the Gren LSP server. These standards directly control code generation and must be followed precisely.

## Core Standards

### Languages & Runtimes
- **Rust**: 2021 Edition minimum
- **Cargo**: Use workspace configuration in lsp-server/
- **VS Code Extension**: TypeScript with strict mode enabled

### Style & Linting
- **rustfmt**: Use default configuration, run `just fmt` before commits
- **clippy**: Zero warnings policy, run `just lint` to verify
- **TypeScript**: ESLint + Prettier for VS Code extension

### Test Organization
- **Unit tests**: In-module using `#[cfg(test)]` blocks
- **Integration tests**: In `lsp-server/tests/` directory
- **Test files**: Use `test_` prefix for functions, `tests.rs` for modules

## Critical Rules

These rules prevent AI agents from introducing bugs or violating architectural principles:

### Logging Requirements
- **NEVER use `println!` or `eprintln!`** - Use `tracing` macros only
- **Required tracing format**: Use structured logging with context
- **Log levels**: Follow exact guidelines from error-handling-strategy.md
- **Performance logging**: Always include timing for LSP operations

### Error Handling Requirements  
- **NEVER use `.unwrap()` or `.expect()`** in production code - Use proper error propagation
- **LSP errors**: Must map to exact LSP error codes using `ResponseError`
- **All functions returning `Result`**: Must use `anyhow::Result` or custom error types
- **Compiler errors**: Parse and map to LSP diagnostics, never ignore

### Symbol Resolution Rules (CRITICAL)
- **NO fuzzy matching** - Exact symbol resolution only using tree-sitter AST
- **NO "close enough" results** - Return empty results instead of approximations  
- **NO fallback mechanisms** - If exact lookup fails, return error
- **Deterministic imports**: Use Gren's explicit import semantics, no path resolution heuristics

### Database Operations
- **NEVER use direct SQL strings** - Use sqlx prepared statements only
- **Required transactions**: All multi-statement operations must use database transactions
- **Connection handling**: Always use async operations via sqlx
- **Migration safety**: All schema changes must be backwards compatible

### Tree-sitter Usage
- **IMPERATIVE: Tree-sitter only** - NO regex or string matching for syntax analysis
- **Query compilation**: Cache compiled queries, never recompile in loops
- **Node traversal**: Use proper AST traversal, check node validity before access
- **Position mapping**: Always use exact UTF-16 calculations for LSP positions

### LSP Protocol Compliance
- **Response format**: All responses must match LSP 3.18 specification exactly
- **Message IDs**: Preserve request/response ID matching
- **Capability negotiation**: Only advertise implemented capabilities
- **UTF-16 encoding**: All position calculations must use UTF-16 code units

## Naming Conventions

### Rust Naming Conventions
| Element | Convention | Example |
|---------|------------|---------|
| **Files** | snake_case | `symbol_index.rs` |
| **Modules** | snake_case | `mod document_manager` |
| **Functions** | snake_case | `find_symbol_definition()` |
| **Structs** | PascalCase | `SymbolIndex` |
| **Enums** | PascalCase | `CompletionKind` |
| **Constants** | SCREAMING_SNAKE_CASE | `DEFAULT_CACHE_SIZE` |
| **Test functions** | snake_case with `test_` prefix | `test_completion_basic()` |

### TypeScript Naming Conventions (VS Code Extension)
| Element | Convention | Example |
|---------|------------|---------|
| **Files** | camelCase or kebab-case | `extension.ts`, `compiler-manager.ts` |
| **Functions** | camelCase | `activateExtension()` |
| **Variables** | camelCase | `languageClient` |
| **Classes** | PascalCase | `CompilerManager` |
| **Interfaces** | PascalCase with I prefix | `ICompilerConfig` |
| **Enums** | PascalCase | `CompilerStatus` |
| **Constants** | SCREAMING_SNAKE_CASE | `DEFAULT_TIMEOUT` |
| **Private members** | camelCase with _ prefix | `_internalState` |

## Language-Specific Guidelines

### Rust Specifics
- **Ownership**: Prefer borrowing over cloning, use `Arc<T>` for shared data
- **Async**: Always use `.await` for async operations, never blocking calls
- **Traits**: Implement `Debug` for all public types, `Clone` when needed
- **Module structure**: Group related functionality, use `pub(crate)` for internal APIs
- **String handling**: Use `&str` for parameters, `String` for owned data, avoid `String::from()`

### Memory Management (Rust)
- **Resource cleanup**: Implement `Drop` trait for resources requiring cleanup
- **String interning**: Use `Arc<str>` for repeated strings in symbol index
- **Cache management**: Respect LRU cache limits, explicit cleanup on document close
- **Database connections**: Use connection pooling, never hold connections across await points

### VS Code Extension Specifics
- **LSP client**: Use official `vscode-languageclient` package
- **Configuration**: All settings must have schema definitions
- **Error handling**: Use VS Code's error reporting mechanisms
- **Activation**: Lazy activation on Gren file open only

## Testing Requirements

### Unit Test Standards
- **Coverage**: All public functions must have unit tests
- **Assertions**: Use exact matching (`assert_eq!`), never approximate (`assert!`)
- **Test data**: Use const strings, no dynamic test data generation
- **Async tests**: Use `#[tokio::test]` for async test functions

### Integration Test Standards  
- **Server lifecycle**: Spawn fresh server process for each test
- **Communication**: Use stdio for LSP communication testing
- **Timeouts**: All async operations must have reasonable timeouts (1000ms default)
- **Test isolation**: No shared state between integration tests

### Test Data Management
- **Fixtures**: Store in `lsp-server/tests/fixtures/`
- **Gren examples**: Use real Gren syntax, not pseudo-code
- **LSP messages**: Store expected JSON in separate files

## Performance Requirements

### Response Time Targets
- **Completion**: < 100ms for 95% of requests
- **Hover**: < 50ms for 95% of requests
- **Go-to-definition**: < 200ms for 95% of requests
- **Find references**: < 200ms for 95% of requests

### Memory Usage
- **Document cache**: LRU with 100 document limit
- **Symbol index**: Efficient cleanup on file changes
- **Tree-sitter**: Reuse parser instances, cache queries

## Security Requirements

### Input Validation
- **All LSP parameters**: Validate using schema before processing
- **File paths**: Validate against workspace bounds, prevent path traversal
- **Compiler input**: Sanitize all data passed to external Gren compiler

### Environment Security
- **GREN_COMPILER_PATH**: Must be explicitly set, validate binary exists and is executable
- **Workspace access**: Limit file operations to workspace directories only
- **External commands**: Only execute configured Gren compiler, no shell commands

## Configuration Management

### Environment Variables
- **Required**: `GREN_COMPILER_PATH` - Fail fast if not set
- **Optional**: `RUST_LOG` - For logging level control
- **Validation**: Check all required environment variables on startup

### Runtime Configuration
- **LSP initialization**: Parse client capabilities and configure features accordingly
- **Workspace settings**: React to workspace/didChangeConfiguration notifications
- **File monitoring**: Monitor `gren.json` for Gren compiler version changes only
- **Document changes**: All other file changes handled via LSP document synchronization messages

## Documentation Requirements

### Code Documentation
- **Public APIs**: All public functions must have `///` documentation
- **Module docs**: Each module must have purpose and usage documentation
- **Error types**: Document when each error variant is returned
- **Examples**: Include usage examples for complex APIs

### Architecture Alignment
- **Reference documents**: All implementations must align with architecture documents
- **Design principles**: Follow "no fallbacks or close enough" principles strictly
- **LSP compliance**: Adhere to LSP 3.18 specification exactly

This coding standards document serves as the primary reference for maintaining code quality and consistency across the Gren LSP server implementation. All code must pass these standards before being considered complete.