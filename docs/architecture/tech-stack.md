# Technology Stack

This document serves as the definitive technology selection reference for the Gren LSP server project. All implementation decisions must align with these choices.

## Core Technology Stack

| Category | Technology | Version | Purpose | Rationale |
|----------|------------|---------|----------|-----------|
| **Language** | Rust | 2021 Edition | Primary development language | Memory safety, performance, excellent async support, strong typing for LSP protocol |
| **Runtime** | Tokio | 1.0+ | Async runtime | Industry-standard async runtime, excellent performance, LSP requires async handling |
| **LSP Framework** | tower-lsp | 0.20 | LSP protocol implementation | Mature LSP server framework, tower middleware support, async-first design |
| **Parser** | tree-sitter | 0.22 | Syntax parsing and AST | Incremental parsing, error recovery, precise syntax analysis, Gren grammar support |
| **Database** | SQLite | via sqlx 0.8 | Symbol indexing storage | Embedded database, no external dependencies, ACID transactions, efficient querying |
| **Serialization** | serde | 1.0 | JSON/LSP protocol | De facto standard, excellent performance, comprehensive derive support |
| **Logging** | tracing | 0.1 | Structured logging | Async-aware, structured logging, excellent ecosystem integration |
| **Error Handling** | anyhow + thiserror | 1.0 | Error management | anyhow for error propagation, thiserror for custom error types |
| **Text Processing** | ropey | 1.6 | Document text handling | Efficient text editing operations, UTF-8 safe, line/column operations |
| **Caching** | lru | 0.12 | Document caching | LRU eviction policy, memory-bounded document cache |

## Development Tools

| Category | Technology | Version | Purpose | Rationale |
|----------|------------|---------|----------|-----------|
| **Build System** | Cargo | Rust standard | Package management | Rust ecosystem standard, excellent dependency management |
| **Task Runner** | just | Latest | Development commands | Simple, cross-platform task runner, better than make |
| **Testing** | tokio-test | 0.4 | Async testing | Async test utilities, integrates with tokio runtime |
| **Linting** | clippy | Rust standard | Code quality | Rust official linter, catches common mistakes and anti-patterns |
| **Formatting** | rustfmt | Rust standard | Code formatting | Consistent code style, Rust community standard |

## Editor Extensions

| Category | Technology | Version | Purpose | Rationale |
|----------|------------|---------|----------|-----------|
| **VS Code Extension** | TypeScript | Latest | Extension implementation | VS Code standard, excellent LSP client support |
| **Extension Framework** | VS Code Extension API | Latest | Editor integration | Direct VS Code integration, comprehensive LSP support |

## Architecture Rationale

### Core Language Choice: Rust
- **Memory Safety**: No runtime exceptions, prevents crashes in editor environment
- **Performance**: Native performance critical for responsive LSP operations
- **Concurrency**: Built-in async/await, essential for handling multiple LSP requests
- **Type Safety**: Strong typing prevents protocol errors and ensures correctness
- **Ecosystem**: Excellent LSP and parsing libraries available

### LSP Framework: tower-lsp
- **Mature**: Battle-tested in production LSP servers
- **Async-First**: Designed for concurrent request handling
- **Standards-Compliant**: Full LSP 3.18 protocol support
- **Extensible**: Tower middleware allows for cross-cutting concerns

### Database: SQLite via sqlx
- **Embedded**: No external database dependencies or setup
- **Performance**: Fast querying for symbol lookup operations
- **ACID**: Ensures symbol index consistency
- **Async**: sqlx provides async database operations
- **Schema Migration**: Built-in migration support for schema evolution

### Parser: tree-sitter
- **Incremental**: Updates only changed parts of AST for performance
- **Error Recovery**: Continues parsing after syntax errors
- **Precise**: Provides exact position information for LSP features
- **Language Support**: Official Gren grammar available

## Version Pinning Strategy

- **Major versions**: Pinned to ensure compatibility
- **Minor versions**: Allow updates for bug fixes and features
- **Security**: Regular dependency auditing via `cargo audit`
- **Testing**: All dependency updates tested in CI/CD pipeline

## External Dependencies

### Required External Tools
- **Gren Compiler**: External binary invoked via `GREN_COMPILER_PATH` environment variable
- **tree-sitter-gren**: Gren language grammar (external crate dependency)

### Development Dependencies
- **VS Code**: For extension development and testing
- **Node.js**: For VS Code extension build toolchain

## Technology Decisions

### Why Not Alternative Frameworks?
- **lsp-server**: More basic, requires manual protocol implementation
- **rust-analyzer architecture**: Too complex for single-language LSP
- **Custom protocol**: LSP provides standardization and editor compatibility

### Database Alternatives Considered
- **In-memory only**: Would lose symbol index between sessions
- **External database**: Adds deployment complexity and dependencies
- **File-based storage**: SQLite provides better querying and consistency

### Parser Alternatives Considered
- **Regex-based**: Insufficient for accurate language analysis
- **Hand-written parser**: Too much maintenance overhead
- **PEG parser**: Less mature ecosystem, no incremental parsing

This technology stack is optimized for:
1. **Developer Experience**: Fast, responsive LSP operations
2. **Maintainability**: Well-established, documented technologies
3. **Performance**: Efficient memory usage and response times
4. **Reliability**: Memory-safe, tested components
5. **Ecosystem Integration**: Standard tools and practices