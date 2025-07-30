# Tech Stack

This section represents the definitive technology selection for the Gren LSP project. Each choice has been evaluated against the project requirements for performance, maintainability, and integration capabilities.

## Cloud Infrastructure

- **Provider:** None (Desktop/Local Development Tool)
- **Key Services:** Local file system, process management, inter-process communication
- **Deployment Regions:** Developer workstations globally

## Technology Stack Table

| Category | Technology | Version | Purpose | Rationale |
|----------|------------|---------|---------|-----------|
| **Core Language** | Rust | 1.70+ | LSP server implementation | Memory safety, zero-cost abstractions, excellent async performance, C interop for Gren compiler |
| **LSP Framework** | tower-lsp | 0.20.0 | Language Server Protocol implementation | Mature Rust LSP framework with async support and comprehensive protocol coverage |
| **Document Management** | lsp-textdocument | 0.3.0 | In-memory document tracking | Handles LSP text synchronization, incremental updates, and version management |
| **Async Runtime** | Tokio | 1.32.0 | Concurrent request handling | Industry standard async runtime, essential for non-blocking LSP operations |
| **Serialization** | serde | 1.0.185 | JSON-RPC protocol handling | Fast, type-safe serialization for LSP message processing |
| **Tree-sitter** | tree-sitter + tree-sitter-gren | Latest | Syntax parsing and AST generation | Fast, incremental parsing with existing Gren grammar support |
| **Database** | SQLite + rusqlite | 1.29.0 | Symbol indexing and caching | Embedded database for persistent symbol storage, no external dependencies |
| **Logging** | tracing | 0.1.37 | Diagnostic logging and performance monitoring | Structured logging with async support for debugging LSP operations |
| **Compiler Integration** | Process execution + output parsing | Limited | Basic diagnostics via compiler output | Phase 1 approach until official Gren compiler API is available |
| **Testing** | cargo test + criterion | 1.5.13 | Unit and performance testing | Built-in Rust testing with benchmarking for performance validation |
| **Build Tool** | Cargo | 1.70+ | Dependency management and builds | Standard Rust build system with excellent ecosystem integration |
| **Editor Extension** | TypeScript | 5.1.6 | VS Code extension development | Required for VS Code LSP client implementation |
