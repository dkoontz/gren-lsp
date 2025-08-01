# Epic 1: Gren LSP Foundation & Tree-sitter Integration

## 🎯 Epic Goal
Establish the foundational infrastructure for Gren LSP server with tree-sitter parsing, basic document management, and core LSP lifecycle to enable editor integration.

## 📝 Epic Description
This epic delivers the minimal viable LSP server that can integrate with editors and provide basic functionality. It focuses on the prerequisite tree-sitter baseline, core LSP service implementation, document synchronization, and basic diagnostics from the Gren compiler.

**Value Delivered**: Developers can open Gren files in LSP-compatible editors and receive basic language server functionality including syntax error reporting and document synchronization.

---

## 📋 User Stories

### [Story 1: Tree-sitter AST Baseline](stories/epic-1-story-1-treesitter-baseline.md)
**Status**: In Progress  
**As a** LSP developer  
**I want** comprehensive tree-sitter AST documentation for all Gren language constructs  
**So that** I can implement accurate parsing and symbol extraction

### [Story 2: Core LSP Service Foundation](stories/epic-1-story-2-lsp-service.md)
**Status**: Pending  
**As a** Gren developer  
**I want** an LSP server that can initialize and communicate with my editor through tested LSP protocol messages  
**So that** I can establish a reliable foundation for language features

### [Story 3: Document Lifecycle Management](stories/epic-1-story-3-document-lifecycle.md)
**Status**: Pending  
**As a** Gren developer  
**I want** my file changes tracked accurately by the LSP server  
**So that** language features work with my current document state

### [Story 4: Basic Compiler Integration & Diagnostics](stories/epic-1-story-4-compiler-integration.md)
**Status**: Pending  
**As a** Gren developer  
**I want** to see syntax and type errors from the Gren compiler in my editor  
**So that** I can fix issues without switching to terminal

---

## 🔗 Epic Dependencies & Integration

### Technical Dependencies
- Tree-sitter Gren grammar (external)
- Gren compiler binary available on system
- Rust toolchain 1.70+

### Integration Points
- Editor LSP clients (VS Code, Neovim, etc.)
- File system for temporary compilation files
- SQLite database for future symbol indexing

### Risk Mitigation
- **Risk**: Tree-sitter grammar parsing issues
  - **Mitigation**: Comprehensive reference file testing, fallback error recovery
- **Risk**: Compiler integration complexity  
  - **Mitigation**: Process isolation, timeout handling, detailed error logging
- **Risk**: Document sync race conditions
  - **Mitigation**: Use proven lsp-textdocument crate, version tracking

---

## ✅ Epic Success Criteria

### Functional Success
- LSP server starts and communicates with editors correctly
- Documents open, edit, and close without data loss
- Compiler diagnostics appear accurately in editor
- No crashes during normal document editing workflow

### Performance Success
- Server startup time < 2 seconds
- Document synchronization latency < 50ms
- Memory usage < 50MB for typical single-file editing

### Quality Success
- Zero data corruption during document editing
- All compiler errors mapped to correct file positions
- Graceful handling of malformed Gren files
- 100% test coverage for implemented LSP message handlers

---

This epic establishes the foundation for all future language features. Once completed, Epic 2 can build core language intelligence features on this solid base.