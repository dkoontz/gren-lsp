# Epic 3 Story 2: Document Symbol Hierarchy

## 📋 User Story
**As a** Gren developer  
**I want** a hierarchical outline of symbols in the current file  
**So that** I can quickly navigate to functions, types, and other declarations within large files

## ✅ Acceptance Criteria
- [ ] Implement textDocument/documentSymbol LSP handler with hierarchical structure
- [ ] Show all symbols in proper hierarchy: Module > Types > Functions > Constants
- [ ] Display symbol kinds correctly (Function, Class, Constructor, Variable, etc.)
- [ ] Provide accurate ranges for symbol selection and navigation
- [ ] Support nested scopes and private declarations
- [ ] Include symbol details (name, kind, range, selectionRange)

## 🧪 Integration Test Requirements

### Test: Symbol Hierarchy Structure
- [ ] Create complex Gren file with module, types, functions, constants
- [ ] Verify hierarchical nesting: Module contains Types and Functions
- [ ] Test that custom types show their constructors as children
- [ ] Validate proper parent-child relationships in symbol tree

### Test: Symbol Kind Classification
- [ ] Verify Module symbols classified as Module kind
- [ ] Test Function symbols classified as Function kind
- [ ] Test Type symbols classified as Class kind (LSP convention)
- [ ] Test Type constructors classified as Constructor kind
- [ ] Test Constants classified as Variable kind
- [ ] Test Import statements handling (if included in outline)

### Test: Range Accuracy
- [ ] Verify symbol ranges span entire declaration (including body)
- [ ] Test selectionRange points to symbol name only
- [ ] Validate ranges don't overlap incorrectly
- [ ] Test range accuracy for complex nested structures

### Test: Complex File Structures
- [ ] Test files with multiple type declarations
- [ ] Test files with nested let expressions
- [ ] Test files with record type definitions
- [ ] Test files with type aliases and their usage

### Test: Edge Cases and Error Handling
- [ ] Test files with syntax errors (partial symbol extraction)
- [ ] Test empty files (should return empty symbol list)
- [ ] Test very large files (performance and memory usage)
- [ ] Test files with complex comment blocks

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC response format matches LSP 3.18 spec
- [ ] Test DocumentSymbol structure with proper nesting
- [ ] Verify symbol kinds use correct LSP SymbolKind values
- [ ] Test proper UTF-16 position calculation for ranges

## 🔧 Technical Implementation

### Symbol Hierarchy Construction
- Build tree structure from flat symbol list
- Determine parent-child relationships using position ranges
- Handle Gren-specific nesting rules (module > type > function)

### Tree-sitter Query Extensions
- Extract symbol hierarchy information during parsing
- Capture parent-child relationships directly from AST
- Handle nested scopes and symbol visibility

### LSP Handler Implementation
- Implement `textDocument/documentSymbol` message handler
- Convert internal symbol representation to LSP DocumentSymbol format
- Support both hierarchical and flat symbol representations

### Gren Language Specifics
- Handle Gren module structure conventions
- Support record types and their field access patterns
- Handle type constructors and pattern matching contexts

## ⚡ Performance Requirements
- Response time: < 100ms for 95% of requests
- Memory usage: Efficient tree construction for large files
- Support files with 1000+ symbols effectively
- Incremental parsing benefits when possible

## ✅ Definition of Done
- textDocument/documentSymbol handler implemented and tested
- Hierarchical symbol structure correctly represents Gren file organization
- All symbol kinds mapped correctly to LSP SymbolKind values
- Range and selectionRange calculations accurate for navigation
- Integration tests validate complex file structures with specific assertions
- Performance requirements met for large files
- Graceful handling of files with syntax errors

## 📁 Related Files
- `src/document_symbols.rs` (TO BE CREATED)
- `gren-lsp-protocol/src/handlers.rs` (TO BE MODIFIED)
- `tests/integration/document_symbols_tests.rs` (TO BE CREATED)
- Integration with existing `src/symbol_index.rs` and `src/tree_sitter_queries.rs`

## 🔗 Dependencies
- Epic 2 Story 1 completed (symbol indexing infrastructure)
- Existing tree-sitter query system for symbol extraction
- LSP message handling framework from Epic 1
- Symbol position and range calculation utilities

## 📊 Status
**Pending** - Ready for Implementation

## 🎯 Success Metrics
- **Navigation Efficiency**: Quick jumping to any symbol in large files
- **Hierarchy Accuracy**: 100% correct parent-child relationships
- **Performance**: Sub-100ms response time for typical files
- **Completeness**: All meaningful symbols included in outline

## 📋 LSP DocumentSymbol Structure

Expected structure for Gren files:
```
Module: "MyModule"
├── Type: "User" 
│   ├── Constructor: "User"
│   └── Field accessors (if applicable)
├── Type: "Status"
│   ├── Constructor: "Active"
│   └── Constructor: "Inactive"
├── Function: "createUser"
├── Function: "updateStatus"
└── Constant: "defaultTimeout"
```

This story addresses the navigation gap identified in the PO Master Checklist, enabling efficient navigation within large Gren files through hierarchical symbol outlines.