# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Language Server Protocol (LSP) implementation for the Gren programming language, built in Rust. Gren is a pure functional programming language forked from Elm, featuring immutable data structures, no runtime exceptions, and array-first data model.

## Common Development Commands



### VS Code Extension
- `just vscode-build` - Build VS Code extension
- `just vscode-package` - Package extension as .vsix
- `just vscode-watch` - Watch extension development

## Architecture

## Development Notes

### Important Development Guidelines
- **ALWAYS use Justfile commands when possible** - Use `just` commands instead of raw `cargo` or `npm` commands to ensure reproducibility of actions and follow project conventions
- **IMPERATIVE: Tree-sitter based implementation** - The LSP implementation MUST be based on tree-sitter parse data and NOT on regex or other string matching techniques. This ensures accurate, incremental parsing that respects Gren's syntax structure
- **CRITICAL: Never lie to the user** - LSP operations that have a single correct answer (like go-to-definition) must either succeed with the correct result or fail/show no result. Due to Gren's deterministic import semantics and absence of polymorphic overloading, there should be almost no "fallback" mechanisms. It's better to show nothing than to show an incorrect result that could confuse developers
- **Clean up temporary scripts** - Always clean up any temporary scripts or files before completing a task

### LSP documentation
The LSP spec is available in the `.docs/lsp-spec/3.18` folder. The documentation is broken out according to message types and pages are linked using `{% include types/uri.md %}` directives that indicate the contents of another file should be inserted at that point in the document.

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
