# Epic 2: Core Language Intelligence Features

## 🎯 Epic Goal
Implement essential language server features that provide immediate developer value: code completion, hover information, and go-to-definition with accurate symbol resolution.

## 📝 Epic Description
This epic transforms the foundation LSP server into a useful development tool by adding the core language intelligence features developers expect. It leverages the tree-sitter parsing and symbol indexing to provide accurate, context-aware assistance.

**Value Delivered**: Developers get intelligent code assistance including autocomplete, type information on hover, and navigation to symbol definitions.

---

## 📋 User Stories

### [Story 1: Symbol Indexing & Cross-Module Resolution](stories/epic-2-story-1-symbol-indexing.md)
**Status**: Pending  
**As a** LSP developer  
**I want** accurate symbol extraction and indexing from Gren code  
**So that** I can provide reliable completion and navigation features

### [Story 2: Code Completion](stories/epic-2-story-2-code-completion.md)
**Status**: Pending  
**As a** Gren developer  
**I want** intelligent code completion suggestions  
**So that** I can write code faster with fewer errors

### [Story 3: Hover Information](stories/epic-2-story-3-hover-information.md)
**Status**: Pending  
**As a** Gren developer  
**I want** to see type information and documentation when hovering over symbols  
**So that** I can understand code without navigating away

### [Story 4: Go-to-Definition](stories/epic-2-story-4-goto-definition.md)
**Status**: Pending  
**As a** Gren developer  
**I want** to navigate directly to symbol definitions  
**So that** I can understand and modify code efficiently

---

## 🔗 Epic Dependencies & Integration

### Prerequisites
- ✅ Epic 1 completed (LSP foundation, document management, tree-sitter baseline)
- ✅ Tree-sitter queries defined and tested for symbol extraction
- ✅ Integration test framework established

### Architecture Alignment
- **Symbol Index** (architecture section 4): SQLite schema implementation
- **Language Feature Handlers** (architecture section 6): Completion, hover, definition handlers
- **Performance Optimizations** (architecture): Caching strategies for symbols and parse trees

### Technical Dependencies
- SQLite database for symbol storage
- Tree-sitter queries for accurate symbol extraction
- Type inference integration with Gren compiler output

---

## ✅ Epic Success Criteria

### Functional Success
- Completion works reliably for common Gren patterns in all major contexts
- **Hover shows accurate type information for 100% of symbols**
- Go-to-definition navigates correctly with 100% precision (no false positives)
- All language features handle Gren's deterministic semantics correctly

### Performance Success
- Completion: < 100ms response time for 95% of requests
- Hover: < 50ms response time for 95% of requests  
- Go-to-definition: < 200ms response time for 95% of requests
- Memory usage remains bounded during intensive symbol operations

### Quality Success
- 100% test coverage for all language feature handlers
- Zero false positives in go-to-definition results
- Symbol index maintains consistency across file changes
- All features work correctly with complex Gren code patterns

---

This epic transforms the basic LSP server into a useful development tool with essential language intelligence features that developers expect from modern language servers.