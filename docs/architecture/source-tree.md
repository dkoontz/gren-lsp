# Source Tree Structure

This document defines the project directory organization and rationale for the Gren LSP server implementation. The structure supports clear component separation and standard development practices.

## Project Root Structure

```
gren-lsp/                           # Project root
├── README.md                       # Main project overview
├── LICENSE                         # MIT license
├── CLAUDE.md                       # AI development instructions
├── justfile                        # Cross-component build commands
├── docs/                           # Project documentation
│   ├── PRD.md                      # Product Requirements Document
│   ├── architecture.md             # Main architecture document
│   ├── architecture/               # Detailed architecture sections
│   │   ├── index.md                # Architecture table of contents
│   │   ├── tech-stack.md           # Technology stack definitions
│   │   ├── coding-standards.md     # Development standards
│   │   ├── source-tree.md          # This document
│   │   ├── core-components.md      # System component architecture
│   │   ├── performance-optimizations.md
│   │   └── [other architecture docs]
│   ├── epics/                      # Epic and story planning
│   │   ├── index.md                # Epic roadmap
│   │   ├── epic-1-foundation.md
│   │   ├── epic-2-core-features.md
│   │   ├── epic-3-advanced-navigation.md
│   │   └── stories/                # Detailed user stories
│   ├── prd/                        # PRD section breakdown
│   ├── lsp-spec/                   # LSP 3.18 specification reference
│   └── tree-sitter-ast/            # Tree-sitter AST examples
├── lsp-server/                     # 🎯 Rust LSP server implementation
│   ├── Cargo.toml                  # Rust project configuration
│   ├── Cargo.lock                  # Dependency lock file
│   ├── src/                        # LSP server source code
│   │   ├── main.rs                 # Entry point
│   │   ├── lib.rs                  # Library exports
│   │   ├── lsp_service.rs          # Core LSP service implementation
│   │   ├── document_manager.rs     # Document lifecycle management
│   │   ├── symbol_index.rs         # SQLite symbol indexing
│   │   ├── tree_sitter_queries.rs  # Tree-sitter query definitions
│   │   ├── completion.rs           # Code completion handler
│   │   ├── hover.rs                # Hover information handler
│   │   ├── goto_definition.rs      # Go-to-definition handler
│   │   ├── diagnostics.rs          # Compiler diagnostic integration
│   │   ├── compiler_interface.rs   # External Gren compiler interface
│   │   ├── gren_language.rs        # Gren language abstraction
│   │   └── scope_analysis.rs       # Symbol scope analysis
│   ├── tests/                      # Integration tests
│   │   ├── integration.rs          # Main integration test runner
│   │   ├── fixtures/               # Test data and LSP messages
│   │   ├── helpers/                # Test utilities and assertions
│   │   └── integration/            # Specific integration test suites
│   └── target/                     # Build artifacts (gitignored)
├── editor-extensions/              # Editor integrations
│   └── vscode/                     # VS Code extension
│       ├── package.json            # Extension manifest
│       ├── src/                    # TypeScript source
│       │   ├── extension.ts        # Main extension logic
│       │   ├── compiler-manager.ts # Gren compiler management
│       │   └── test/               # Extension tests
│       ├── out/                    # Compiled JavaScript
│       └── test-workspace/         # Test Gren project
├── tree-sitter-gren/              # Tree-sitter grammar (external)
│   ├── Cargo.toml                  # Grammar crate configuration
│   ├── grammar.js                  # Grammar definition
│   ├── src/                        # Generated parser code
│   ├── queries/                    # Syntax highlighting queries
│   └── test/                       # Grammar test corpus
└── dev-tools/                     # Development utilities
    └── test-data/                  # Test fixtures and samples
        ├── gren-example-projects/  # Complete Gren projects for testing
        │   ├── application/        # Simple application example
        │   └── package/            # Complex package example
        ├── gren-samples/           # Simple Gren test files
        ├── lsp-messages/           # LSP protocol test messages
        └── lsp-test-cases.md       # Test case documentation
```

## Component Organization Rationale

### 1. lsp-server/ - Rust LSP Implementation
**Purpose**: Contains the complete Rust LSP server implementation following standard Rust project conventions.

**Key Design Decisions**:
- **Standard Rust layout**: `src/`, `tests/`, `Cargo.toml` follow Rust ecosystem conventions
- **Feature-based modules**: Each LSP feature (completion, hover, etc.) has its own module
- **Clear separation**: LSP protocol handling separate from language-specific logic
- **Integration tests**: Full LSP protocol testing in dedicated `tests/` directory

**Module Dependencies**:
```
main.rs → lsp_service.rs → {completion, hover, goto_definition, diagnostics}
                         ↓
                    document_manager.rs ← symbol_index.rs
                         ↓
                tree_sitter_queries.rs ← gren_language.rs
                         ↓
                   compiler_interface.rs
```

### 2. editor-extensions/ - Editor Integrations
**Purpose**: Contains editor-specific extensions that integrate with the LSP server.

**Structure Rationale**:
- **Extensible**: Easy to add new editors (neovim/, emacs/, etc.)
- **Self-contained**: Each extension is a complete, buildable project
- **Standard patterns**: Follows each editor's extension conventions

### 3. tree-sitter-gren/ - Language Grammar
**Purpose**: External dependency containing the Gren language grammar for tree-sitter.

**Integration Notes**:
- **External crate**: Referenced as path dependency in lsp-server/Cargo.toml
- **Self-contained**: Can be developed and tested independently
- **Standard grammar structure**: Follows tree-sitter conventions

### 4. dev-tools/test-data/ - Development Resources
**Purpose**: Consolidated test data and development utilities.

**Organization Benefits**:
- **Single source**: All test data in one location
- **Reusable**: Test projects can be used across different test suites
- **Realistic**: Contains actual Gren projects, not toy examples

## Build System Integration

### Justfile Commands
The root `justfile` provides unified commands that work across components:

```bash
# LSP Server (Rust)
just build           # cd lsp-server && cargo build
just test            # cd lsp-server && cargo test
just run             # cd lsp-server && cargo run

# VS Code Extension
just vscode-build    # cd editor-extensions/vscode && npm run compile
just vscode-dev      # Build and install extension for testing

# Cross-component
just vscode-dev      # Builds LSP server + installs VS Code extension
```

### Workspace Configuration
- **Rust workspace**: `lsp-server/` is the primary Rust workspace
- **VS Code workspace**: Extension has independent TypeScript build
- **Test coordination**: Integration tests can spawn LSP server and test via VS Code

## File Naming and Organization Conventions

### Rust Code Organization
- **Feature modules**: Each LSP feature gets its own `.rs` file
- **Integration tests**: Group related tests in `tests/integration/` subdirectories
- **Test helpers**: Shared utilities in `tests/helpers/`

### Documentation Organization
- **Reference docs**: Technical specifications in `docs/architecture/`
- **Planning docs**: Epics and stories for project management
- **Specifications**: External specs (LSP) for reference

### Test Data Organization
- **Complete projects**: Full Gren projects in `gren-example-projects/`
- **Simple samples**: Individual files in `gren-samples/`
- **Protocol data**: LSP messages in `lsp-messages/`

## Development Workflow Support

### Local Development
1. **LSP server**: Work in `lsp-server/` with standard Rust tools
2. **Extension**: Work in `editor-extensions/vscode/` with TypeScript tools
3. **Testing**: Use `just vscode-dev` to test complete integration

### CI/CD Support
- **Multiple workspaces**: Each component can be built independently
- **Cross-component testing**: Integration tests verify complete functionality
- **Artifact organization**: Clear separation of build outputs

## Scalability Considerations

### Adding New Features
- **LSP features**: Add new module in `lsp-server/src/`
- **Editor support**: Add new directory under `editor-extensions/`
- **Test data**: Add examples to appropriate `dev-tools/test-data/` subdirectory

### Performance Isolation
- **Build caching**: Each component builds independently
- **Test isolation**: Integration tests don't interfere with unit tests
- **Development speed**: Can work on individual components without full rebuild

This source tree structure supports the project's goals of clear component separation, standard development practices, and scalability for future enhancements while maintaining the quality and organization needed for professional LSP server development.