# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 BEFORE YOU START: Essential Test Projects

**CRITICAL**: This project includes comprehensive Gren test projects at `dev-tools/test-data/gren-example-projects/` and `dev-tools/test-data/gren-samples/`. These MUST be used for ALL testing and development work instead of creating temporary test files. See the "Test Projects and Data" section below for full details.

## Project Overview

This is a Language Server Protocol (LSP) implementation for the Gren programming language, built in Rust. Gren is a pure functional programming language forked from Elm, featuring immutable data structures, no runtime exceptions, and array-first data model.

## Project Structure

The project is organized into clear components:

- **`lsp-server/`** - Rust LSP server implementation
  - `src/` - LSP server source code
  - `tests/` - Integration tests
  - `Cargo.toml` - Rust project configuration
- **`editor-extensions/`** - Editor integrations
  - `vscode/` - VS Code extension
- **`tree-sitter-gren/`** - Tree-sitter grammar for Gren
- **`docs/`** - Project documentation, architecture, epics
- **`dev-tools/test-data/`** - Test fixtures and sample projects
  - `gren-example-projects/` - Sample Gren projects for testing
  - `gren-samples/` - Simple Gren test files
  - `lsp-messages/` - LSP protocol test messages

## Common Development Commands

### LSP Server (Rust)
- `just build` - Build LSP server (equivalent to `cd lsp-server && cargo build`)
- `just test` - Run all tests
- `just test-integration` - Run integration tests only
- `just run` - Run LSP server
- `just run-debug` - Run with debug logging
- `just check` - Check code without building
- `just fmt` - Format code
- `just lint` - Run clippy linter

### VS Code Extension
- `just vscode-build` - Build VS Code extension
- `just vscode-package` - Package extension as .vsix
- `just vscode-watch` - Watch extension development
- `just vscode-dev` - Build LSP server and install VS Code extension

## Architecture

## Development Notes

### Important Development Guidelines
- **ALWAYS use Justfile commands when possible** - Use `just` commands instead of raw `cargo` or `npm` commands to ensure reproducibility of actions and follow project conventions
- **IMPERATIVE: Tree-sitter based implementation** - The LSP implementation MUST be based on tree-sitter parse data and NOT on regex or other string matching techniques. This ensures accurate, incremental parsing that respects Gren's syntax structure
- **CRITICAL: Never lie to the user** - LSP operations that have a single correct answer (like go-to-definition) must either succeed with the correct result or fail/show no result. Due to Gren's deterministic import semantics and absence of polymorphic overloading, there should be almost no "fallback" mechanisms. It's better to show nothing than to show an incorrect result that could confuse developers
- **MANDATORY: Use existing test projects** - ALWAYS use the pre-built Gren test projects in `dev-tools/test-data/` for testing and development. DO NOT create temporary test files - use the existing comprehensive test suite that covers real Gren project structures
- **Clean up temporary scripts** - Always clean up any temporary scripts or files before completing a task

### LSP documentation
The LSP spec is available in the `docs/lsp-spec/3.18` folder. The documentation is broken out according to message types and pages are linked using `{% include types/uri.md %}` directives that indicate the contents of another file should be inserted at that point in the document.

### Gren Language Characteristics
- Pure functional with no exceptions (uses Maybe/Result types)
- Array-first data structures (not lists)
- Immutable by default
- Effects handled through Cmd types
- Small, predictable syntax ideal for tree-sitter parsing
- **Deterministic imports**: Explicit import statements with no ambiguity - each symbol has exactly one source
- **No polymorphic overloading**: Function names are unique within their scope, enabling precise symbol resolution
- **No tuples**: Gren uses records instead of tuples. Where Elm uses `( Model, Cmd Msg )`, Gren uses `{ model : Model, command : Cmd Msg }`
- **Pattern matching**: Uses `when` keyword instead of `case` for pattern matching expressions

### Performance Considerations
- Uses LRU caching for workspace documents (default 100 items)
- SQLite database for persistent symbol indexing
- Async processing to prevent editor blocking
- Incremental parsing with tree-sitter for efficiency

### Debugging and Logging
- **VS Code Extension Debug Log**: The VS Code extension automatically creates a debug log file that captures both extension and LSP server output
  - **Location**: `{OS_TEMP_DIR}/gren-lsp/debug.log` (e.g., `/tmp/gren-lsp/debug.log` on macOS/Linux, `%TEMP%\gren-lsp\debug.log` on Windows)
  - **Content**: Contains timestamped logs from both the VS Code extension and LSP server with source prefixes `[Extension]` and `[LSP Server]`
  - **Lifecycle**: Log file is cleared on each extension activation to prevent accumulation
  - **Access**: File path is displayed in VS Code's "Gren LSP" output channel when the extension starts
  - **Usage**: Use this file for debugging LSP communication issues, server startup problems, and protocol message tracing

## Current Project Status

**📍 For current epic progress and story status, see the canonical source:**
- **Epic Overview & Status**: [`docs/epics/index.md`](docs/epics/index.md)
- **Story Files**: [`docs/stories/`](docs/stories/) directory
- **Next Story**: Use the BMad Scrum Master (`*sm`) to identify and prepare the next story

This approach prevents documentation drift by maintaining a single source of truth for project status and progress tracking.

## 🧪 CRITICAL: Test Projects and Data

**MANDATORY FOR ALL DEVELOPMENT** - This project includes comprehensive pre-built Gren test projects that MUST be used for all testing, development, and validation work.

### Primary Test Projects
- **`dev-tools/test-data/gren-example-projects/`** - **USE THESE FOR ALL TESTING**
  - `application/` - Complete Gren application with realistic project structure
  - `package/` - Complex Gren package with multiple modules, imports, and dependencies
  - These projects contain real Gren syntax, proper module structures, and representative codebases

### Secondary Test Data  
- **`dev-tools/test-data/gren-samples/`** - Simple, focused Gren test files for specific feature testing
- **`dev-tools/test-data/lsp-messages/`** - LSP protocol test messages and mock data

### IMPORTANT: Development Requirements
- **DO NOT create temporary test files** - Use the existing comprehensive test projects
- **DO NOT use `tempfile::TempDir` for Gren project testing** - Use the real projects in `dev-tools/test-data/`
- **For integration tests**: Always point to existing test projects rather than creating synthetic ones
- **For module rename/refactoring**: Use the multi-module structure in `gren-example-projects/package/`
- **For basic LSP features**: Use the simple structure in `gren-example-projects/application/`

These test projects ensure realistic testing conditions and prevent developers from creating artificial test scenarios that don't reflect real Gren usage patterns.
