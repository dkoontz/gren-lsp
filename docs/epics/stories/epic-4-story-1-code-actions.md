# Epic 4 Story 1: Code Actions for Common Fixes

## 📋 User Story
**As a** Gren developer  
**I want** automated code actions for common compiler errors and improvements  
**So that** I can quickly fix issues and improve code quality without manual typing

## ✅ Acceptance Criteria
- [ ] Implement textDocument/codeAction LSP handler with contextual suggestions
- [ ] Provide "Add missing import" actions for undefined symbols
- [ ] Suggest "Fix type annotation" actions for type mismatches
- [ ] Offer "Remove unused import" actions for cleanup
- [ ] Support "Add type signature" actions for functions missing annotations
- [ ] Provide proper LSP CodeAction structure with edit commands
- [ ] Include diagnostic-based actions triggered by compiler errors
- [ ] Support cursor position-based actions for code improvements

## 🧪 Integration Test Requirements

### Test: Missing Import Code Actions
- [ ] Create Gren file using undefined symbol from known module
- [ ] Verify code action suggests "Import Foo exposing (bar)"
- [ ] Test that applying action adds correct import statement
- [ ] Validate import is added in proper location (after existing imports)

### Test: Type Annotation Code Actions
- [ ] Create function without type signature
- [ ] Verify code action suggests "Add type signature"
- [ ] Test that applying action adds inferred type signature
- [ ] Validate signature is syntactically correct and properly formatted

### Test: Unused Import Cleanup
- [ ] Create file with unused imports
- [ ] Verify code action suggests "Remove unused import"
- [ ] Test that applying action removes only unused imports
- [ ] Validate remaining imports are preserved correctly

### Test: Type Mismatch Fixes
- [ ] Create code with type mismatch compiler error
- [ ] Verify code action suggests appropriate type conversion
- [ ] Test actions like "Convert to String" or "Wrap in Maybe"
- [ ] Validate suggested fixes resolve the type error

### Test: Multiple Actions per Diagnostic
- [ ] Create code with multiple possible fixes
- [ ] Verify multiple code actions are offered
- [ ] Test that each action addresses the issue differently
- [ ] Validate actions don't conflict with each other

### Test: Cursor-Based Actions
- [ ] Position cursor on function without type signature
- [ ] Verify code actions available even without diagnostic
- [ ] Test position-sensitive action suggestions
- [ ] Validate actions are relevant to cursor context

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC response format matches LSP 3.18 spec
- [ ] Test CodeAction structure with proper title, kind, and edit
- [ ] Verify WorkspaceEdit applies changes correctly
- [ ] Test proper UTF-16 position encoding for edits

## 🔧 Technical Implementation

### Code Action Categories
- **quickfix**: Fix compiler errors (missing imports, type mismatches)
- **refactor.rewrite**: Improve code structure (add type signatures)
- **source.organizeImports**: Clean up import statements
- **source.fixAll**: Apply all available quick fixes

### Diagnostic Integration
- Parse compiler error messages for actionable issues
- Map diagnostic ranges to potential code actions
- Provide contextual suggestions based on error type
- Support batch fixes for multiple similar issues

### LSP Handler Implementation
- Implement `textDocument/codeAction` message handler
- Support both diagnostic-triggered and cursor-based actions
- Generate WorkspaceEdit operations for applying fixes
- Handle action preferences and filtering by client

### Gren Language Specifics
- Handle Gren import syntax and module resolution
- Support Gren type system patterns and conversions
- Work with Gren's deterministic import semantics
- Generate syntactically correct Gren code modifications

## ⚡ Performance Requirements
- Response time: < 100ms for 95% of requests
- Support files with 50+ potential actions efficiently
- Minimize compiler invocations for action generation
- Cache action suggestions when possible

## ✅ Definition of Done
- textDocument/codeAction handler implemented and tested
- Code actions provide helpful fixes for common compiler errors
- Actions generate syntactically correct Gren code
- WorkspaceEdit operations apply changes accurately
- Integration tests validate all action categories with specific assertions
- Performance requirements met for typical development scenarios
- Error handling for invalid or conflicting actions

## 📁 Related Files
- `src/code_actions.rs` - Main CodeActionsEngine implementation
- `src/lsp_service.rs` - LSP handler integration and capability advertisement
- `src/code_actions_integration_tests.rs` - Comprehensive test coverage
- Integration with existing `src/compiler_integration.rs` and `src/symbol_index.rs`

## 🔗 Dependencies
- Epic 1-2 completed (LSP foundation, compiler integration, symbol indexing)
- Existing diagnostic system for error-based actions
- Symbol index for import suggestion resolution
- Tree-sitter queries for code structure analysis

## 📊 Status
**⏳ PENDING** - Ready for implementation

## 🎯 Success Metrics
- **Developer Productivity**: 80% of common errors fixable via code actions
- **Accuracy**: 100% syntactically correct generated code
- **Performance**: Sub-100ms response time for action suggestions
- **Coverage**: Support for top 10 most common Gren compiler errors

## 💡 Code Action Examples

### Missing Import
```gren
-- Before (with error)
main = Html.text "Hello"

-- Code Action: "Import Html"
-- After
import Html
main = Html.text "Hello"
```

### Add Type Signature  
```gren
-- Before
add x y = x + y

-- Code Action: "Add type signature"
-- After
add : Int -> Int -> Int
add x y = x + y
```

### Remove Unused Import
```gren
-- Before
import Json.Decode
import Html

main = Html.text "Hello"

-- Code Action: "Remove unused import Json.Decode"
-- After  
import Html

main = Html.text "Hello"
```

This story addresses the productivity gap by providing automated fixes for common development scenarios, reducing manual typing and helping developers learn proper Gren patterns through suggested improvements.