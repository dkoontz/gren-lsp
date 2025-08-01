# Project Background

## About Gren
Gren is a pure functional programming language forked from Elm, featuring:
- Immutable data structures with no runtime exceptions
- Array-first data model (instead of lists)
- Deterministic import semantics with explicit import statements
- No polymorphic overloading - function names are unique within scope
- Small, predictable syntax ideal for tree-sitter parsing
- Effects handled through Cmd types

## Technical Context
- **Implementation Language**: Rust
- **LSP Framework**: async-lsp crate (asynchronous, tower-based)
- **Parsing Strategy**: Tree-sitter based (not regex/string matching)
- **Compilation**: External Gren compiler invoked via environment variable
- **Communication**: JSON-RPC over stdio interface
