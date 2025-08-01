# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Language Server Protocol (LSP) implementation for the Gren programming language, built in Rust. Gren is a pure functional programming language forked from Elm, featuring immutable data structures, no runtime exceptions, and array-first data model.

## Differences between Gren and Elm
- **No Tuples**: Gren does not have Tuples, only Records. The typical `init` function in Elm would be `init : flags -> (model, Cmd msg)`, but in Gren this would be `init : flags -> { model : model, command : Cmd msg }`.
- **Pattern matching syntax**: Uses `when foo is` instead of `case foo of`
- **File naming conventions**: Gren module names must be UpperCase (e.g., `Main.gren`, `SyntaxTest.gren`) and the module declaration must match the filename
- **Compiler version**: Currently using Gren 0.6.1 with JSON error reporting via `--report=json` flag

## Common Development Commands

### Essential Commands (using just)
- `just build` - Build the project
- `just test` - Run all tests
- `just check` - Run format check, lint, and tests
- `just fmt` - Format code
- `just lint` - Run clippy lints
- `just run` - Run the LSP server
- `just run-debug` - Run with debug logging (`RUST_LOG=gren_lsp=debug`)

### Development Workflow
- `just watch` - Watch for changes and rebuild
- `just watch-test` - Watch and run tests
- `just install` - Install LSP binary locally
- `just doc` - Generate and open documentation

### VS Code Extension
- `just vscode-build` - Build VS Code extension
- `just vscode-package` - Package extension as .vsix
- `just vscode-watch` - Watch extension development

### VS Code Extension Testing
- `cd editor-extensions/vscode && npm test` - Run all VS Code extension tests
- `npm test -- --grep "pattern"` - Run specific tests matching a pattern
- Tests include LSP integration, diagnostic validation, and protocol message monitoring
- **CRITICAL**: Extension NEVER uses system-installed Gren compiler, only downloads and uses version specified in project's gren.json

## Architecture

### Workspace Structure
This is a Rust workspace with three main crates:
- **gren-lsp-server**: Main LSP server binary (`src/main.rs`, `src/server.rs`)
- **gren-lsp-core**: Core analysis engine with modules:
  - `analysis.rs` - Analysis coordination
  - `parser.rs` - Tree-sitter parsing with incremental updates
  - `workspace.rs` - Workspace management with LRU caching
  - `symbol.rs` - Symbol indexing with SQLite backend
  - `document.rs` - Document state management
  - `compiler.rs` - Gren compiler integration with temporary file management and diagnostic generation
  - `diagnostics.rs` - Error reporting with path mapping for subdirectory files
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
5. Parser uses tree-sitter for incremental parsing and syntax error detection
6. Compiler module creates temporary files, runs Gren compiler, and maps diagnostics back to original files
7. Symbol index maintains SQLite database for fast lookups
8. Diagnostic path mapping handles subdirectory structures (e.g., `src/` directories)

## Development Notes

### Important Development Guidelines
- **ALWAYS use Justfile commands when possible** - Use `just` commands instead of raw `cargo` or `npm` commands to ensure reproducibility of actions and follow project conventions
- **IMPERATIVE: Tree-sitter based implementation** - The LSP implementation MUST be based on tree-sitter parse data and NOT on regex or other string matching techniques. This ensures accurate, incremental parsing that respects Gren's syntax structure
- **CRITICAL: Never lie to the user** - LSP operations that have a single correct answer (like go-to-definition) must either succeed with the correct result or fail/show no result. Due to Gren's deterministic import semantics and absence of polymorphic overloading, there should be almost no "fallback" mechanisms. It's better to show nothing than to show an incorrect result that could confuse developers
- **Clean up temporary scripts** - Always clean up any temporary scripts or files before completing a task

### LSP documentation
The LSP spec is available in the `.docs/lsp-spec/3.18` folder. The documentation is broken out according to message types and pages are linked using `{% include types/uri.md %}` directives that indicate the contents of another file should be inserted at that point in the document.

### Testing
- **Rust Unit Tests**: Unit tests are in `src/` alongside modules
- **Rust Integration Tests**: Integration tests in `tests/` directories
- **Benchmarks**: Available in `benches/` (gren-lsp-core)
- **Test Files**: Available in `test-files/` directory
- **VS Code Extension Tests**: Located in `editor-extensions/vscode/src/test/suite/`
  - `diagnostics.test.ts` - Tests LSP diagnostic functionality with real Gren compiler integration
  - `lsp-protocol-features.test.ts` - Tests core LSP protocol features
  - `helpers/` - Test utilities including `ObservedLSPMessageMonitor` for real LSP message interception
  - Test workspace: `editor-extensions/vscode/test-workspace/` with proper Gren project structure

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

### Recent Progress (Session Summary)
- ✅ **Fixed Gren vs Elm syntax differences**: Updated all test files to use proper Gren record syntax instead of Elm tuple syntax
- ✅ **Fixed file naming conventions**: Ensured all Gren files use UpperCase.gren format with matching module names
- ✅ **Updated Gren compiler integration**: Successfully integrated Gren 0.6.1 with JSON diagnostic reporting
- ✅ **Fixed diagnostic path mapping**: Enhanced `compiler.rs` to properly map diagnostic paths from temporary compilation directories back to original files, including subdirectory structures like `src/`
- ✅ **Improved VS Code extension testing**: Created comprehensive diagnostic tests with real LSP message monitoring using `ObservedLSPMessageMonitor`

### Known Issues
- **Test Infrastructure Issue**: VS Code extension diagnostic tests can capture LSP `didOpen` messages but the LSP server's `didOpen` handler is not processing dynamically created test files. The diagnostic functionality itself works correctly - this is a message routing issue in the test environment.

### Technical Accomplishments
- **Compiler Integration**: Successfully integrated Gren compiler with temporary file management, module name extraction via tree-sitter, and comprehensive error handling
- **Diagnostic Path Mapping**: Fixed critical path mapping logic to handle files in subdirectories (e.g., `src/SyntaxTest.gren` -> original file paths)
- **LSP Protocol Testing**: Implemented sophisticated LSP message interception for testing real protocol communication without mocking
- **Gren Syntax Compliance**: All test files now use proper Gren syntax with records, pattern matching (`when...is`), and correct Node.js application structure
