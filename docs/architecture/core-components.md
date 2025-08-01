# Core Components

## 1. LSP Service Layer

**Framework**: async-lsp crate + lsp-types crate
- **LspService Implementation**: Core service trait handling LSP requests and notifications
- **Message Router**: Dispatches incoming JSON-RPC messages to appropriate handlers
- **Type System**: Uses lsp-types for all LSP protocol structures (requests, responses, notifications)
- **Capability Manager**: Negotiates and tracks client/server capabilities using `ClientCapabilities` and `ServerCapabilities`
- **Error Handler**: Manages LSP-level errors using `lsp_types::error_codes` constants

**Key Characteristics**:
- Asynchronous request handling for concurrent processing
- Synchronous notification handling to maintain correct ordering
- Tower-based middleware support for cross-cutting concerns
- Built-in JSON-RPC 2.0 protocol handling
- Type-safe LSP protocol implementation with compile-time guarantees

**Type Integration**:
```rust
use lsp_types::{
    ClientCapabilities, ServerCapabilities, InitializeParams, InitializeResult,
    CompletionParams, CompletionResponse, HoverParams, Hover,
    DefinitionParams, DefinitionResponse, Position, Range, TextDocumentItem,
    DidOpenTextDocumentParams, DidChangeTextDocumentParams, PublishDiagnosticsParams,
};
use async_lsp::{LspService, ClientSocket, LanguageServer};
```

## 2. Document Manager

**Framework**: lsp-textdocument crate + custom extensions
- **TextDocuments**: Manages document lifecycle and incremental synchronization
- **FullTextDocument**: Handles individual document content and UTF-16 position encoding
- **Tree-sitter Integration**: Custom layer for syntax tree management
- **Position Conversion**: Automatic handling of offset-to-position mappings

**Responsibilities**:
- Track all open text documents with their content and versions
- Apply incremental and full document changes using proven algorithms
- Maintain document state consistency with UTF-16 position encoding
- Provide document access and position calculations to other components
- Cache parse trees and invalidate them on content changes

**Implementation Details**:
```rust
use lsp_types::{Url, TextDocumentItem, VersionedTextDocumentIdentifier, TextDocumentContentChangeEvent};
use lsp_textdocument::{TextDocuments, FullTextDocument};

pub struct DocumentManager {
    text_documents: TextDocuments,
    parse_trees: HashMap<Url, Tree>, // Tree-sitter parse trees
    lru_cache: LruCache<Url, FullTextDocument>, // For closed documents
    parser: Parser, // Tree-sitter parser
}

impl DocumentManager {
    pub fn new() -> Self {
        Self {
            text_documents: TextDocuments::new(),
            parse_trees: HashMap::new(),
            lru_cache: LruCache::new(100),
            parser: Parser::new(),
        }
    }

    pub fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<(), DocumentError> {
        let uri = &params.text_document.uri;

        // Let lsp-textdocument handle the document lifecycle
        self.text_documents.listen(lsp_types::notification::DidOpenTextDocument::METHOD, params)?;

        // Parse the initial content
        self.update_parse_tree(uri)?;

        Ok(())
    }

    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<(), DocumentError> {
        let uri = &params.text_document.uri;

        // Let lsp-textdocument handle incremental updates
        self.text_documents.listen(lsp_types::notification::DidChangeTextDocument::METHOD, params)?;

        // Update parse tree incrementally if possible
        self.update_parse_tree(uri)?;

        Ok(())
    }

    pub fn get_document_content(&self, uri: &Url) -> Option<String> {
        self.text_documents.get_document_content(uri, None)
    }

    pub fn get_position_from_offset(&self, uri: &Url, offset: usize) -> Option<Position> {
        let document = self.text_documents.get_document(uri)?;
        document.position_at(offset)
    }

    pub fn get_offset_from_position(&self, uri: &Url, position: Position) -> Option<usize> {
        let document = self.text_documents.get_document(uri)?;
        document.offset_at(position)
    }

    pub fn get_parse_tree(&self, uri: &Url) -> Option<&Tree> {
        self.parse_trees.get(uri)
    }

    fn update_parse_tree(&mut self, uri: &Url) -> Result<(), DocumentError> {
        let content = self.get_document_content(uri).ok_or(DocumentError::NotFound)?;

        if let Some(old_tree) = self.parse_trees.get(uri) {
            // Use incremental parsing when possible
            let new_tree = self.parser.parse(&content, Some(old_tree))?;
            self.parse_trees.insert(uri.clone(), new_tree);
        } else {
            // Full parse for new documents
            let tree = self.parser.parse(&content, None)?;
            self.parse_trees.insert(uri.clone(), tree);
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("Document not found")]
    NotFound,
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Position encoding error")]
    PositionError,
}
```

**Features**:
- **Proven Document Management**: Uses lsp-textdocument's battle-tested incremental update algorithms
- **UTF-16 Position Encoding**: Automatic handling of LSP's UTF-16 position requirements
- **Offset/Position Conversion**: Built-in conversion between byte offsets and line/column positions
- **LRU cache for recently closed documents** (default: 100 items)
- **Atomic document updates** to prevent race conditions
- **Incremental parsing with tree-sitter** for optimal performance
- **Version tracking** to handle out-of-order messages

**Advantages of lsp-textdocument Integration**:
- **Reliability**: Based on VS Code's text document implementation
- **Performance**: Optimized incremental text synchronization algorithms
- **Standards Compliance**: Handles LSP specification nuances correctly
- **Position Mapping**: Eliminates common bugs in UTF-16 offset calculations
- **Maintenance**: Delegates complex document state management to a dedicated library

## 3. Tree-sitter Parser

**Purpose**: Provide structural analysis of Gren code without regex patterns

**Implementation Prerequisites**:
Before implementing any tree-sitter functionality, a baseline AST capture must be established:

1. **Create Comprehensive Test File**: Develop a complete Gren source file (`docs/tree-sitter-ast/reference.gren`) that exercises all language constructs:
   - Module declarations and imports
   - Function definitions with type annotations
   - Custom type definitions and variants
   - Record types and record updates
   - Pattern matching with all patterns
   - Let expressions and local bindings
   - Case expressions with guards
   - Comments (single-line and multi-line)
   - String literals and interpolation
   - Number literals (integers, floats, hex)
   - Array and record literals
   - Pipe operators and function composition
   - All operators and precedence levels

2. **Generate Reference AST**: Use tree-sitter CLI to parse the reference file and capture the complete AST structure:
   ```bash
   tree-sitter parse docs/tree-sitter-ast/reference.gren > docs/tree-sitter-ast/baseline.ast
   ```

3. **Document AST Structure**: Create `docs/tree-sitter-ast/README.md` explaining:
   - Node types and their purposes
   - Field names and their meanings  
   - Query patterns for common language constructs
   - Expected AST patterns for each language feature

**Components**:
- **Parser Instance**: Tree-sitter parser configured for Gren grammar
- **Query Engine**: Execute tree-sitter queries for symbol extraction
- **Incremental Updates**: Apply changes efficiently to existing parse trees
- **Error Recovery**: Handle partial/invalid syntax gracefully

**Core Queries**:
```scheme
;; Function definitions
(function_declaration
  name: (identifier) @function.name
  parameters: (parameter_list) @function.params
  body: (_) @function.body)

;; Import statements
(import_declaration
  module: (module_name) @import.module
  exposing: (exposing_list) @import.exposing)

;; Type definitions
(type_declaration
  name: (identifier) @type.name
  variants: (_) @type.variants)
```

## 4. Symbol Index

**Storage**: SQLite database for persistent symbol information

**Schema**:
```sql
CREATE TABLE symbols (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    kind INTEGER NOT NULL,
    uri TEXT NOT NULL,
    range_start_line INTEGER NOT NULL,
    range_start_char INTEGER NOT NULL,
    range_end_line INTEGER NOT NULL,
    range_end_char INTEGER NOT NULL,
    container TEXT,
    signature TEXT,
    documentation TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_symbols_name ON symbols(name);
CREATE INDEX idx_symbols_uri ON symbols(uri);
CREATE INDEX idx_symbols_kind ON symbols(kind);
```

**Operations**:
- **Indexing**: Extract symbols from parse trees and store in database
- **Querying**: Fast symbol lookups by name, location, or type
- **Updates**: Incremental updates when files change
- **Cross-references**: Track symbol relationships across modules

## 5. Compiler Interface

**Integration Model**: External process invocation
- **Process Management**: Spawn and manage Gren compiler processes
- **Temporary Files**: Write in-memory documents to temp files for compilation
- **Output Parsing**: Parse compiler JSON output for diagnostics
- **Error Handling**: Robust handling of compiler failures

**Implementation**:
```rust
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

struct CompilerInterface {
    compiler_path: PathBuf,
    temp_dir: TempDir,
    process_pool: Pool<CompilerProcess>,
}

struct CompileRequest {
    file_path: PathBuf,
    content: String,
    project_root: PathBuf,
}

struct CompileResult {
    success: bool,
    diagnostics: Vec<Diagnostic>, // Uses lsp_types::Diagnostic
    symbols: Vec<Symbol>,
    dependencies: Vec<ModuleDependency>,
}

impl CompileResult {
    fn to_publish_diagnostics(&self, uri: Url, version: Option<i32>) -> PublishDiagnosticsParams {
        PublishDiagnosticsParams {
            uri,
            version,
            diagnostics: self.diagnostics.clone(),
        }
    }
}
```

## 6. Language Feature Handlers

All handlers use lsp-types for type-safe parameter handling and response construction.

### Completion Handler
```rust
use lsp_types::{CompletionParams, CompletionResponse, CompletionItem, CompletionItemKind, MarkupContent};

async fn handle_completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>, LspError> {
    let position = params.text_document_position.position;
    let uri = params.text_document_position.text_document.uri;

    // Trigger analysis, scope resolution, import analysis, filtering
    let items = self.get_completion_items(&uri, position).await?;

    Ok(Some(CompletionResponse::Array(items)))
}
```

### Hover Handler
```rust
use lsp_types::{HoverParams, Hover, MarkupContent, MarkupKind};

async fn handle_hover(&self, params: HoverParams) -> Result<Option<Hover>, LspError> {
    let position = params.text_document_position_params.position;
    let uri = params.text_document_position_params.text_document.uri;

    // Symbol resolution, type inference, documentation lookup
    if let Some((content, range)) = self.get_hover_info(&uri, position).await? {
        Ok(Some(Hover {
            contents: lsp_types::HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: Some(range),
        }))
    } else {
        Ok(None)
    }
}
```

### Definition Handler
```rust
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, LocationLink};

async fn handle_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>, LspError> {
    let position = params.text_document_position_params.position;
    let uri = params.text_document_position_params.text_document.uri;

    // Symbol identification, cross-module resolution, package resolution
    let locations = self.find_definitions(&uri, position).await?;

    if locations.is_empty() {
        Ok(None)
    } else {
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }
}
```

### References Handler
```rust
use lsp_types::{ReferenceParams, Location};

async fn handle_references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>, LspError> {
    let position = params.text_document_position.position;
    let uri = params.text_document_position.text_document.uri;
    let include_declaration = params.context.include_declaration;

    // Symbol usage analysis, scope analysis, package boundaries
    let references = self.find_references(&uri, position, include_declaration).await?;

    if references.is_empty() {
        Ok(None)
    } else {
        Ok(Some(references))
    }
}
```

### Diagnostics Handler
```rust
use lsp_types::{PublishDiagnosticsParams, Diagnostic, DiagnosticSeverity};

async fn publish_diagnostics(&self, uri: Url, version: Option<i32>) {
    // Compiler integration, error classification, range mapping
    let diagnostics = self.compute_diagnostics(&uri).await.unwrap_or_default();

    let params = PublishDiagnosticsParams {
        uri,
        version,
        diagnostics,
    };

    self.client.publish_diagnostics(params).await;
}
```
