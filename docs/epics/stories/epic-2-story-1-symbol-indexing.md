# Epic 2 Story 1: Symbol Indexing & Cross-Module Resolution

## 📋 User Story
**As a** LSP developer  
**I want** accurate symbol extraction and indexing from Gren code  
**So that** I can provide reliable completion and navigation features

## ✅ Acceptance Criteria
- [ ] Symbol Index implemented with SQLite database (schema from architecture)
- [ ] Tree-sitter queries extract functions, types, imports, variables from AST
- [ ] Cross-module symbol resolution using Gren's deterministic imports
- [ ] Incremental symbol updates when files change (only reindex affected files)
- [ ] Symbol relationships tracked (what imports what, where symbols are defined)

## 🧪 Integration Test Requirements

### Test: Symbol Extraction Accuracy
- [ ] Parse complex Gren files with all language constructs
- [ ] Verify all symbols extracted with correct names, types, positions
- [ ] Test extraction from reference file created in Epic 1
- [ ] Test symbol extraction from nested scopes and modules

### Test: Cross-Module Resolution
- [ ] Create multi-file test project with imports
- [ ] Verify symbols resolved to correct source modules
- [ ] Test import chain resolution (A imports B imports C)
- [ ] Test handling of circular import detection

### Test: Incremental Index Updates
- [ ] Modify file and verify only affected symbols reindexed
- [ ] Test that index remains consistent after updates
- [ ] Verify no memory leaks during repeated updates
- [ ] Test concurrent update handling

### Test: Database Schema and Operations
- [ ] Test SQLite database schema matches architecture specification
- [ ] Test all CRUD operations on symbol index
- [ ] Test database performance with large symbol sets
- [ ] Test database recovery from corruption

## ✅ Definition of Done
- SQLite database schema matches architecture specification
- All Gren language constructs properly indexed
- Cross-module resolution works with 100% accuracy
- Index updates complete within 100ms for typical file changes
- Symbol relationships correctly tracked and queryable

## 📁 Related Files
- `src/symbol_index.rs` (TO BE CREATED)
- `src/tree_sitter_queries.rs` (TO BE CREATED)
- `tests/integration/symbol_indexing_tests.rs` (TO BE CREATED)
- Database schema from `docs/architecture/core-components.md`

## 🔗 Dependencies
- Epic 1 completed (LSP foundation, document management, tree-sitter baseline)
- SQLite database
- Tree-sitter queries for symbol extraction
- Cross-module dependency tracking

## 📊 Status
**Pending** - Awaiting Epic 1 completion