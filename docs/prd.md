# Gren Language Server Protocol (LSP) Implementation - Product Requirements Document

## Executive Summary

This document outlines the requirements for implementing a Language Server Protocol (LSP) server for the Gren programming language using Rust and the async-lsp crate. The LSP server will provide essential language features including completion, hover information, go-to-definition, references, diagnostics, and document symbols to editors and IDEs that support the LSP standard.

## Project Background

### About Gren
Gren is a pure functional programming language forked from Elm, featuring:
- Immutable data structures with no runtime exceptions
- Array-first data model (instead of lists)
- Deterministic import semantics with explicit import statements
- No polymorphic overloading - function names are unique within scope
- Small, predictable syntax ideal for tree-sitter parsing
- Effects handled through Cmd types

### Technical Context
- **Implementation Language**: Rust
- **LSP Framework**: async-lsp crate (asynchronous, tower-based)
- **Parsing Strategy**: Tree-sitter based (not regex/string matching)
- **Compilation**: External Gren compiler invoked via environment variable
- **Communication**: JSON-RPC over stdio interface

## Goals and Objectives

### Primary Goals
1. **Editor Integration**: Enable Gren support in VS Code, Neovim, Emacs, and other LSP-compatible editors
2. **Developer Experience**: Provide fast, accurate language features that enhance productivity
3. **Reliability**: Implement robust error handling and never provide incorrect results
4. **Performance**: Ensure responsive operation even on large Gren codebases

### Success Metrics
- Sub-100ms response time for completion and hover requests
- 100% accuracy for go-to-definition (no false positives)
- Support for workspaces with 100+ Gren files
- Zero crashes during normal operation

## Functional Requirements

### Core Language Server Features

#### 1. Server Lifecycle Management
- **Initialize/Initialized**: Establish connection and negotiate capabilities
- **Shutdown/Exit**: Graceful server termination
- **Capability Negotiation**: Advertise supported features to client

#### 2. Document Synchronization
- **didOpen**: Track newly opened documents
- **didChange**: Apply incremental or full document updates
- **didClose**: Clean up closed document state
- **Document Versioning**: Maintain correct version tracking for all documents

#### 3. Language Intelligence Features

##### Completion (textDocument/completion)
- **Module Member Completion**: Suggest available functions/types from imported modules
- **Local Variable Completion**: Suggest variables in current scope
- **Keyword Completion**: Suggest Gren language keywords
- **Trigger Characters**: Support completion on "." character
- **Rich Completion Items**: Include type signatures and documentation

##### Hover Information (textDocument/hover)
- **Type Information**: Display inferred or annotated types
- **Documentation**: Show module documentation for functions
- **Import Information**: Indicate which module provides a symbol
- **Range Highlighting**: Highlight the relevant symbol range

##### Go-to-Definition (textDocument/definition)
- **Local Definitions**: Navigate to function/variable definitions in same file
- **Cross-Module Definitions**: Navigate to definitions in other project files
- **Package Definitions**: Navigate to definitions in installed packages
- **Precise Results**: Never return multiple results for unambiguous symbols

##### Find References (textDocument/references)
- **Local References**: Find all uses within the same file
- **Cross-Module References**: Find uses across the entire project
- **Include Declaration**: Option to include/exclude the definition location
- **Accurate Results**: All results must be actual references (no false positives)

##### Document Symbols (textDocument/documentSymbol)
- **Hierarchical Structure**: Show module structure with nested functions/types
- **Symbol Types**: Correctly classify modules, functions, types, etc.
- **Navigation Support**: Provide ranges for quick navigation within document

#### 4. Diagnostics (textDocument/publishDiagnostics)
- **Syntax Errors**: Report parsing failures with precise locations
- **Type Errors**: Report type mismatches from compiler
- **Import Errors**: Report missing or incorrect imports
- **Naming Errors**: Report undefined variables/functions
- **Real-time Updates**: Publish diagnostics on document changes
- **Clear Diagnostics**: Remove diagnostics when issues are resolved

### Advanced Features (Future Phases)
- **Workspace Symbols**: Search across entire project
- **Code Actions**: Suggest fixes for common errors
- **Rename**: Safe symbol renaming across project
- **Formatting**: Code formatting using Gren formatter

## Technical Requirements

### Architecture Constraints

#### LSP Framework
- Use async-lsp crate for asynchronous message handling
- Implement LspService trait for both server and potential client functionality
- Leverage tower middleware for common LSP features
- Handle notifications synchronously to maintain correct ordering

#### Tree-sitter Integration
- All parsing must use tree-sitter, not regex or string matching
- Implement incremental parsing for performance
- Maintain parse trees for all open documents
- Use tree-sitter queries for symbol extraction

#### Compiler Integration
- Invoke external Gren compiler specified by environment variable
- Write in-memory document states to temporary files for compilation
- Parse compiler output for diagnostic information
- Cache compilation results when possible

#### Performance Requirements
- **Response Times**:
  - Completion: < 100ms for 95% of requests
  - Hover: < 50ms for 95% of requests
  - Go-to-definition: < 200ms for 95% of requests
  - Diagnostics: < 500ms after document change
- **Memory Usage**: < 100MB for typical projects (< 50 files)
- **Startup Time**: < 2 seconds for project initialization

#### Reliability Requirements
- **Error Handling**: Never crash on malformed input
- **Graceful Degradation**: Provide partial results when possible
- **Accurate Results**: Prefer no result over incorrect result
- **State Consistency**: Maintain correct document state across all operations

### Data Management

#### Document Storage
- Maintain in-memory copies of all open documents
- Apply incremental changes correctly
- Track document versions to prevent race conditions
- Implement LRU cache for closed documents (default 100 items)

#### Symbol Indexing
- Use SQLite database for persistent symbol storage
- Index all project symbols at startup
- Incrementally update index on file changes
- Support cross-module symbol resolution

#### Workspace Management
- Support single-folder and multi-folder workspaces
- Detect Gren project structure (gren.json, src/ directory)
- Handle workspace configuration changes
- Monitor file system changes for non-open files

## Non-Functional Requirements

### Security
- Validate all input from clients
- Prevent path traversal attacks in file operations
- Limit resource consumption to prevent DoS
- Never execute arbitrary code from documents

### Compatibility
- Support LSP specification version 3.18
- Work with VS Code, Neovim, Emacs, and other LSP clients
- Handle client capability variations gracefully
- Maintain backward compatibility with LSP 3.15+

### Maintainability
- Comprehensive test coverage (>80%)
- Clear separation of concerns between LSP handling and language logic
- Extensive logging for debugging
- Configuration through environment variables

### Deployment
- Single binary distribution
- No external dependencies beyond Gren compiler
- Cross-platform support (Windows, macOS, Linux)
- Integration with VS Code extension

## User Experience Requirements

### Editor Integration
- Seamless installation through package managers
- Automatic startup when opening Gren files
- Status indicators for server health
- Configuration options for advanced users

### Development Workflow
- Real-time feedback as code is typed
- Accurate suggestions that improve productivity
- Fast navigation between definitions
- Clear error messages with helpful suggestions

### Error Handling
- Informative error messages for common issues
- Graceful handling of invalid Gren code
- Recovery from temporary compiler failures
- User-friendly diagnostics with actionable suggestions

## Implementation Phases

### Phase 1: Foundation (MVP)
**Timeline**: 4-6 weeks
**Deliverables**:
- Tree-sitter baseline AST capture and documentation (prerequisite)
- Basic LSP server lifecycle (initialize, shutdown, exit)
- Document synchronization (didOpen, didChange, didClose)
- Basic diagnostics from compiler output
- Tree-sitter integration for parsing

**Success Criteria**:
- Complete AST baseline captured and documented in `docs/tree-sitter-ast/`
- Server starts and shuts down correctly
- Documents sync properly with no data loss
- No crashes during normal operation

### Phase 2: Core Language Features
**Timeline**: 6-8 weeks
**Deliverables**:
- Code completion for modules and local symbols
- Hover information with type signatures
- Go-to-definition for local and cross-module symbols
- Enhanced diagnostics with type errors

**Success Criteria**:
- Completion works reliably for common patterns
- Hover shows accurate type information
- Go-to-definition navigates correctly 90% of the time
- Type errors displayed with helpful messages

### Phase 3: Advanced Navigation
**Timeline**: 4-6 weeks
**Deliverables**:
- Find all references functionality
- Document symbol outline
- Performance optimizations
- Enhanced error recovery

**Success Criteria**:
- References found accurately across project
- Symbol outline provides useful navigation
- Performance meets stated requirements
- Handles large projects (100+ files) effectively

### Phase 4: Polish and Enhancement
**Timeline**: 4-6 weeks
**Deliverables**:
- Code actions for common fixes
- Workspace symbols search
- Rename functionality
- Comprehensive documentation

**Success Criteria**:
- Code actions provide useful suggestions
- Workspace search finds symbols quickly
- Rename works safely across project
- Documentation supports user adoption

### Phase 5: VS Code Extension Integration
**Timeline**: 2-3 weeks
**Deliverables**:
- VS Code extension package
- Extension marketplace publication
- User installation guide
- Extension configuration options

**Success Criteria**:
- Extension installs and activates correctly
- LSP server starts automatically when opening Gren files
- All language features work seamlessly in VS Code
- User-friendly configuration interface

## Risk Assessment

### Technical Risks
1. **Tree-sitter Complexity**: Learning curve for tree-sitter implementation
   - **Mitigation**: Start with simple queries, iterate gradually
2. **Compiler Integration**: External process coordination challenges
   - **Mitigation**: Robust process management, error handling
3. **Performance**: Meeting response time requirements
   - **Mitigation**: Early performance testing, optimization focus

### Project Risks
1. **Scope Creep**: Adding features beyond core requirements
   - **Mitigation**: Strict phase gating, clear success criteria
2. **Testing Complexity**: LSP protocol testing challenges
   - **Mitigation**: Comprehensive test framework, JSON-RPC validation
3. **Client Compatibility**: Variations in LSP client implementations
   - **Mitigation**: Test with multiple editors, conservative feature use

## Success Criteria

### Functional Success
- All Phase 1-2 features working correctly
- 100% accuracy for go-to-definition requests
- Completion suggestions relevant and helpful
- Zero data loss during document synchronization

### Performance Success
- Response times meet specified requirements
- Memory usage within acceptable bounds
- Handles typical project sizes without issues
- Startup time acceptable for development workflow

### User Adoption Success
- Positive feedback from early adopters
- Integration with popular editors
- Active usage by Gren community
- Minimal support requests due to bugs

This LSP implementation will significantly improve the development experience for Gren programmers by bringing modern IDE features to a language designed for reliability and developer productivity.
