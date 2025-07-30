# Data Models

The core data models represent the fundamental entities that the LSP server manages during analysis and interaction with editors.

## Workspace Model

**Purpose:** Represents the complete project workspace including all Gren source files, dependencies, and analysis state

**Key Attributes:**
- `root_path`: PathBuf - Root directory of the workspace
- `source_files`: HashMap<PathBuf, SourceFile> - All tracked Gren source files
- `dependencies`: Vec<Dependency> - External package dependencies
- `analysis_cache`: AnalysisCache - Cached analysis results for performance
- `symbol_index`: SymbolIndex - Global symbol database for cross-file operations

**Relationships:**
- Contains multiple SourceFile entities
- Manages dependency resolution across modules
- Maintains bidirectional references for efficient navigation

## SourceFile Model

**Purpose:** Represents a single Gren source file with its content, AST, and analysis metadata

**Key Attributes:**
- `text_document`: lsp_textdocument::FullTextDocument - LSP document with version tracking and incremental updates
- `path`: PathBuf - File system path (derived from text_document.uri)
- `ast`: TreeSitterAST - Tree-sitter parsed syntax tree
- `diagnostics`: Vec<Diagnostic> - Error and warning messages
- `symbols`: Vec<Symbol> - Symbols defined in this file
- `last_analyzed`: SystemTime - Last analysis timestamp

**Relationships:**
- Belongs to parent Workspace
- References symbols defined in other files through imports
- Contains multiple Symbol entities

## Symbol Model

**Purpose:** Represents language symbols (functions, types, variables) with their definitions and usage information, optimized for Gren's pure functional programming model

**Key Attributes:**
- `name`: String - Symbol identifier
- `kind`: GrenSymbolKind - Function, UnionType, TypeAlias, Variable, Module, etc.
- `location`: Location - Source position (file, line, column)
- `type_signature`: Option<GrenTypeSignature> - Type information including Maybe/Result patterns and Cmd types
- `documentation`: Option<String> - Associated documentation
- `visibility`: Visibility - Public, private, or module-internal
- `references`: Array<Location> - All usage locations
- `returns_cmd`: bool - Whether function returns a Cmd type for runtime effect execution
- `immutability`: bool - Whether symbol represents immutable data (always true for Gren values)

**Relationships:**
- Defined within a specific SourceFile
- May reference other symbols through type dependencies
- Tracked across files for go-to-definition and find-references

## Diagnostic Model

**Purpose:** Represents compiler errors, warnings, and suggestions with precise location information

**Key Attributes:**
- `severity`: DiagnosticSeverity - Error, Warning, Information, Hint
- `range`: Range - Source code range (start/end positions)
- `message`: String - Human-readable diagnostic message
- `code`: Option<String> - Compiler-specific error code
- `source`: String - Always "gren-lsp" for our diagnostics
- `related_information`: Vec<DiagnosticRelatedInformation> - Additional context

**Relationships:**
- Associated with specific SourceFile
- May reference related symbols for context
- Linked to compiler analysis results
