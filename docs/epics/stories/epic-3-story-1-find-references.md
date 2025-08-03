# Epic 3 Story 1: Find All References Implementation

## 📋 User Story
**As a** Gren developer  
**I want** to find all usages of a symbol across my entire project  
**So that** I can understand symbol usage, refactor safely, and navigate my codebase efficiently

## ✅ Acceptance Criteria
- [ ] Implement textDocument/references LSP handler with 100% accuracy
- [ ] Find all symbol references across multiple files in the workspace
- [ ] Support both local and cross-module reference finding
- [ ] Include/exclude declaration location based on includeDeclaration parameter
- [ ] Leverage Gren's deterministic semantics for zero false positives
- [ ] Handle all symbol types: functions, types, variables, constants

## 🧪 Integration Test Requirements

### Test: Local References Accuracy
- [ ] Create test file with local function references in different scopes
- [ ] Verify all local usages found with correct positions
- [ ] Test references in let expressions, function parameters, and nested scopes
- [ ] Validate zero false positives for similar named symbols

### Test: Cross-Module References
- [ ] Create multi-file test project with cross-module symbol usage
- [ ] Verify references found across import boundaries
- [ ] Test imported symbol references with aliases
- [ ] Test references to symbols in nested modules

### Test: Include/Exclude Declaration
- [ ] Test includeDeclaration=true returns definition location
- [ ] Test includeDeclaration=false excludes definition location
- [ ] Verify behavior matches LSP specification exactly
- [ ] Test with both local and cross-module definitions

### Test: Symbol Type Coverage
- [ ] Test function references (local and imported)
- [ ] Test type references (custom types, type aliases)
- [ ] Test variable and constant references
- [ ] Test module references in import statements

### Test: Edge Cases and Error Handling
- [ ] Test references in files with syntax errors
- [ ] Test references to non-existent symbols (should return empty)
- [ ] Test performance with large files (>1000 lines)
- [ ] Test concurrent reference requests

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC message format matches LSP 3.18 spec
- [ ] Test proper error responses for invalid requests
- [ ] Verify location ranges are accurate and complete
- [ ] Test workspace folder handling for multi-folder projects

## 🔧 Technical Implementation

### Database Schema Extensions
- Extend symbol index to track reference relationships
- Index symbol usage positions for fast lookup
- Store import/export relationships for cross-module resolution

### Tree-sitter Query Extensions
- Create queries to identify symbol references in all contexts
- Handle variable references, function calls, type annotations
- Extract position information for accurate location reporting

### LSP Handler Implementation
- Implement `textDocument/references` message handler
- Integrate with existing symbol index from Epic 2
- Support workspace-wide reference searching
- Handle includeDeclaration parameter correctly

## ⚡ Performance Requirements
- Response time: < 200ms for 95% of requests
- Memory usage: Bounded during reference searching
- Support projects with 100+ files effectively
- Incremental updates maintain reference accuracy

## ✅ Definition of Done
- textDocument/references handler implemented and tested
- All symbol types supported with 100% accuracy
- Cross-module references work correctly with imports
- includeDeclaration parameter handled per LSP spec
- Integration tests cover all acceptance criteria with specific assertions
- Performance requirements met for typical Gren projects
- Zero false positives (leveraging Gren's deterministic semantics)

## 📁 Related Files
- `src/find_references.rs` (TO BE CREATED)
- `gren-lsp-protocol/src/handlers.rs` (TO BE MODIFIED)
- `tests/integration/references_tests.rs` (TO BE CREATED)
- Integration with existing `src/symbol_index.rs`

## 🔗 Dependencies
- Epic 2 Story 1 completed (symbol indexing infrastructure)
- Existing tree-sitter query system
- SQLite symbol database operational
- LSP message handling framework from Epic 1

## 📊 Status
**Pending** - Ready for Implementation

## 🎯 Success Metrics
- **Accuracy**: 100% precision (no false positives) leveraging Gren's deterministic import semantics
- **Coverage**: All symbol references found across workspace
- **Performance**: Sub-200ms response time for typical projects
- **Reliability**: Handles edge cases gracefully without crashes

This story completes the essential "Find References" functionality that was identified as the highest priority missing feature in the PO Master Checklist validation.