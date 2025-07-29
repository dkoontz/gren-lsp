# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Language Server Protocol (LSP) implementation for the Gren programming language, built in Rust. Gren is a pure functional programming language forked from Elm, featuring immutable data structures, no runtime exceptions, and array-first data model.

## Common Development Commands

### Essential Commands (using just)
- `just build` - Build the project
- `just test` - Run all tests  
- `just check` - Run format check, lint, and tests
- `just fmt` - Format code
- `just lint` - Run clippy lints
- `just run` - Run the LSP server
- `just run-debug` - Run with debug logging (`RUST_LOG=gren_lsp=debug`)
- `just ci` - Run all CI checks locally (check + audit)

### Development Workflow
- `just watch` - Watch for changes and rebuild
- `just watch-test` - Watch and run tests
- `just install` - Install LSP binary locally
- `just doc` - Generate and open documentation

### VS Code Extension
- `just vscode-build` - Build VS Code extension
- `just vscode-package` - Package extension as .vsix
- `just vscode-watch` - Watch extension development

## Architecture

### Workspace Structure
This is a Rust workspace with three main crates:
- **gren-lsp-server**: Main LSP server binary (`src/main.rs`, `src/server.rs`)
- **gren-lsp-core**: Core analysis engine with modules:
  - `analysis.rs` - Analysis coordination
  - `parser.rs` - Tree-sitter parsing
  - `workspace.rs` - Workspace management with LRU caching
  - `symbol.rs` - Symbol indexing with SQLite backend
  - `document.rs` - Document state management
  - `diagnostics.rs` - Error reporting
- **gren-lsp-protocol**: LSP protocol handlers (`handlers.rs`)

### Key Dependencies
- `tower-lsp` - LSP framework
- `tree-sitter` - Incremental parsing
- `rusqlite` - Symbol database
- `tokio` - Async runtime
- `tracing` - Structured logging

### Data Flow
1. Editor communicates via LSP protocol through `gren-lsp-server`
2. Server delegates to protocol handlers in `gren-lsp-protocol`
3. Core analysis happens in `gren-lsp-core` components
4. Workspace manages documents with LRU caching
5. Parser uses tree-sitter for incremental parsing
6. Symbol index maintains SQLite database for fast lookups

## Development Notes

### Important Development Guidelines
- **ALWAYS use Justfile commands when possible** - Use `just` commands instead of raw `cargo` or `npm` commands to ensure reproducibility of actions and follow project conventions
- **IMPERATIVE: Tree-sitter based implementation** - The LSP implementation MUST be based on tree-sitter parse data and NOT on regex or other string matching techniques. This ensures accurate, incremental parsing that respects Gren's syntax structure
- **CRITICAL: Never lie to the user** - LSP operations that have a single correct answer (like go-to-definition) must either succeed with the correct result or fail/show no result. Due to Gren's deterministic import semantics and absence of polymorphic overloading, there should be almost no "fallback" mechanisms. It's better to show nothing than to show an incorrect result that could confuse developers
- **Clean up temporary scripts** - Always clean up any temporary scripts or files before completing a task

### Testing
- Unit tests are in `src/` alongside modules
- Integration tests in `tests/` directories
- Benchmarks in `benches/` (gren-lsp-core)
- Test files available in `test-files/` directory

### Gren Language Characteristics
- Pure functional with no exceptions (uses Maybe/Result types)
- Array-first data structures (not lists)
- Immutable by default
- Effects handled through Cmd types
- Small, predictable syntax ideal for tree-sitter parsing
- **Deterministic imports**: Explicit import statements with no ambiguity - each symbol has exactly one source
- **No polymorphic overloading**: Function names are unique within their scope, enabling precise symbol resolution

### Performance Considerations
- Uses LRU caching for workspace documents (default 100 items)
- SQLite database for persistent symbol indexing
- Async processing to prevent editor blocking
- Incremental parsing with tree-sitter for efficiency

### Toolchain
- Rust 1.82.0 (specified in rust-toolchain.toml)
- Uses workspace dependencies for version management
- Release profile optimized for performance (LTO enabled)

## Current Development Status

The project follows epic-based development with documentation in `docs/epics/`. Core LSP functionality is being implemented with tree-sitter parsing as a foundation for eventual full Gren compiler integration.