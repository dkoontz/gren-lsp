# External APIs

## Gren Compiler Integration

**Purpose:** Direct integration with Gren compiler for AST parsing, type checking, and error reporting

**Documentation:** Gren compiler source code and internal APIs (no external URL - direct source integration)
**Base URL(s):** N/A (FFI integration)
**Authentication:** N/A (local process integration)
**Rate Limits:** N/A (local calls)

**Key Integration Points:**
- `parse_module(source: &str) -> Result<AST, ParseError>` - Parse Gren source to AST
- `type_check_module(ast: AST, context: TypeContext) -> TypeResult` - Perform type analysis
- `get_module_dependencies(module: &Module) -> Vec<ModuleName>` - Extract import dependencies

**Integration Notes:** Requires careful memory management at FFI boundary, error handling for malformed source, and efficient serialization of AST structures

## Language Server Protocol Specification

**Purpose:** Adherence to LSP specification for editor communication standards

**Documentation:** https://microsoft.github.io/language-server-protocol/
**Base URL(s):** JSON-RPC over stdio/TCP
**Authentication:** N/A (process communication)
**Rate Limits:** N/A (local communication)

**Key Protocol Methods:**
- `initialize` - Server capability negotiation
- `textDocument/completion` - Code completion requests
- `textDocument/definition` - Go-to-definition requests
- `textDocument/publishDiagnostics` - Error reporting to client

**Integration Notes:** Must maintain protocol version compatibility, handle client capability differences, implement proper error responses

## Tree-sitter Grammar Integration

**Purpose:** Fast, incremental parsing using existing tree-sitter-gren grammar

**Documentation:** https://github.com/MaeBrooks/tree-sitter-gren
**Base URL(s):** N/A (embedded library)
**Authentication:** N/A
**Rate Limits:** N/A

**Key Integration Points:**
- `ts_parser_parse()` - Parse Gren source with tree-sitter
- `ts_tree_get_changed_ranges()` - Efficient incremental parsing
- Custom query patterns for symbol extraction and syntax highlighting

**Integration Notes:** Grammar is still in development by MaeBrooks. May require contributions or local modifications for complete Gren language support. This provides the foundation for Phase 1 implementation while preparing for eventual full compiler integration.
