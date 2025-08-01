# Epic 2 Story 3: Hover Information

## 📋 User Story
**As a** Gren developer  
**I want** to see type information and documentation when hovering over symbols  
**So that** I can understand code without navigating away

## ✅ Acceptance Criteria
- [ ] **Type Information**: Display inferred or annotated types for all symbols
- [ ] **Documentation**: Extract and display module documentation comments
- [ ] **Import Source**: Show which module provides the symbol
- [ ] **Range Highlighting**: Highlight the exact symbol being hovered
- [ ] **Markdown Formatting**: Proper formatting for documentation display

## 🧪 Integration Test Requirements

### Test: Type Information Accuracy
- [ ] Test hover on functions shows correct type signatures
- [ ] Test hover on variables shows inferred types
- [ ] Test hover on custom types shows type definition
- [ ] Test hover on record fields shows field types

### Test: Documentation Extraction
- [ ] Test hover shows documentation from `{-| ... -}` comments
- [ ] Test documentation formatting in hover response
- [ ] Test handling of missing documentation
- [ ] Test multi-line documentation handling

### Test: Symbol Range Accuracy
- [ ] Test hover range matches exact symbol boundaries
- [ ] Test hover on qualified names (Module.function)
- [ ] Test hover doesn't activate on whitespace/comments
- [ ] Test hover range precision with complex expressions

### Test: Performance Requirements
- [ ] All hover requests respond within 50ms
- [ ] Test performance with large files and complex types
- [ ] Test hover response caching effectiveness
- [ ] Test memory usage during intensive hover operations

### Test: Import Source Attribution
- [ ] Test hover shows correct module source for imported symbols
- [ ] Test hover distinguishes local vs imported symbols
- [ ] Test hover with re-exported symbols
- [ ] Test hover with transitive imports

## ✅ Definition of Done
- **Hover shows accurate type information for 100% of symbols**
- Documentation extracted and formatted correctly
- Response time consistently < 50ms for 95% of requests
- Hover range precisely matches symbol boundaries
- All symbol types handled correctly (functions, types, variables)

## 📁 Related Files
- `src/hover.rs` (TO BE CREATED)
- `src/type_inference.rs` (TO BE CREATED)
- `src/documentation_extraction.rs` (TO BE CREATED)
- `tests/integration/hover_tests.rs` (TO BE CREATED)

## 🔗 Dependencies
- Epic 2 Story 1 completed (symbol indexing)
- Type inference system
- Documentation parsing
- Symbol range calculation

## 📊 Status
**Pending** - Awaiting Story 1 completion