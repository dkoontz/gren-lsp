# Epic 4 Story 3: Safe Symbol Rename

## 📋 User Story
**As a** Gren developer  
**I want** to safely rename symbols across my entire project  
**So that** I can refactor code with confidence that all references are updated correctly

## ✅ Acceptance Criteria
- [ ] Implement textDocument/rename LSP handler with workspace-wide rename capability
- [ ] Find all references to symbol before applying rename (100% accuracy required)
- [ ] Support renaming functions, types, modules, variables, and type constructors
- [ ] Validate new name follows Gren naming conventions and doesn't create conflicts
- [ ] Apply rename across all files simultaneously with proper transaction semantics
- [ ] Provide preview of changes before applying (via prepareRename if supported)
- [ ] Handle complex rename scenarios (module renames affecting imports)
- [ ] Ensure compilation succeeds after rename operation

## 🧪 Integration Test Requirements

### Test: Local Symbol Rename
- [ ] Rename function within single file
- [ ] Verify all local references updated correctly
- [ ] Test rename of function parameters and local variables
- [ ] Validate type signature references are updated

### Test: Cross-Module Function Rename
- [ ] Rename exported function used in multiple modules
- [ ] Verify import statements updated correctly
- [ ] Test qualified references (Module.function) are updated
- [ ] Validate unqualified references in importing modules

### Test: Type Rename with Constructors
- [ ] Rename custom type and verify constructor references updated
- [ ] Test type annotation updates across modules
- [ ] Verify pattern matching expressions are updated
- [ ] Test type alias references are handled correctly

### Test: Module Rename
- [ ] Rename module file and verify all import statements updated
- [ ] Test qualified references (Module.symbol) across project
- [ ] Verify file path changes are handled correctly
- [ ] Test nested module renames affect qualified imports

### Test: Rename Validation
- [ ] Test rename to existing symbol name (should reject)
- [ ] Verify invalid Gren identifiers are rejected
- [ ] Test rename that would create shadowing conflicts
- [ ] Validate reserved keyword conflicts are detected

### Test: Complex Rename Scenarios
- [ ] Rename symbol that appears in comments (should preserve)
- [ ] Test rename of symbol used in string literals (should preserve)
- [ ] Verify record field renames update field access patterns
- [ ] Test type constructor renames in pattern matching

### Test: Rename Transaction Semantics
- [ ] Verify all-or-nothing behavior for multi-file renames
- [ ] Test that partial failures don't leave project in inconsistent state
- [ ] Validate rollback capability if compilation fails
- [ ] Test concurrent edit scenarios

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC response format matches LSP 3.18 spec
- [ ] Test WorkspaceEdit structure with proper file changes
- [ ] Verify TextEdit ranges are accurate and non-overlapping
- [ ] Test prepareRename support for rename preview

## 🔧 Technical Implementation

### Reference Resolution
- Leverage existing Find References implementation for comprehensive symbol finding
- Extend symbol resolution to handle all symbol types (functions, types, modules)
- Implement precise symbol matching to avoid false positives
- Handle qualified vs unqualified references correctly

### Rename Validation Engine
- Check new name against Gren naming conventions
- Validate no conflicts with existing symbols in scope
- Detect potential shadowing issues
- Verify new name doesn't conflict with reserved keywords

### Workspace Edit Generation
- Generate TextEdit operations for all symbol references
- Handle import statement updates for module/symbol renames
- Support file renames for module rename operations
- Ensure proper ordering of edit operations

### Compilation Validation
- Perform dry-run compilation after generating rename edits
- Validate that renamed code compiles successfully
- Provide rollback mechanism if compilation fails
- Generate helpful error messages for rename conflicts

## ⚡ Performance Requirements
- Response time: < 2 seconds for 95% of symbol renames in typical projects
- Memory usage: Efficient handling of large rename operations
- Support projects with 1000+ references to renamed symbol
- Minimize compilation overhead during validation

## ✅ Definition of Done
- textDocument/rename handler implemented and tested
- Rename operations maintain 100% compilation success rate
- All symbol references found and updated accurately across workspace
- Proper validation prevents invalid or conflicting renames
- Integration tests validate complex rename scenarios with specific assertions
- Performance requirements met for large-scale rename operations
- Graceful error handling and rollback for failed operations

## 📁 Related Files
- `src/rename.rs` - Main RenameEngine implementation
- `src/lsp_service.rs` - LSP handler integration and capability advertisement  
- `src/rename_integration_tests.rs` - Comprehensive test coverage
- Integration with existing `src/find_references.rs` for reference resolution
- Extensions to `src/symbol_index.rs` for rename validation
- Integration with `src/compiler_integration.rs` for validation

## 🔗 Dependencies
- Epic 3 Story 1 completed (Find References implementation)
- Epic 2 Story 1 completed (Symbol indexing for validation)
- Existing symbol resolution and workspace management
- Compiler integration for post-rename validation

## 📊 Status
**⏳ PENDING** - Ready for implementation

## 🎯 Success Metrics
- **Accuracy**: 100% of rename operations maintain compilation success
- **Coverage**: All symbol references found and updated correctly
- **Safety**: Zero cases of missed references or invalid renames
- **Performance**: Sub-2-second response time for typical rename operations

## 🔄 Rename Examples

### Function Rename
```gren
-- Before
-- File: src/User.gren
createUser : String -> User
createUser name = { name = name }

-- File: src/Main.gren  
import User exposing (createUser)
main = createUser "Alice"

-- Rename: createUser → makeUser
-- After
-- File: src/User.gren
makeUser : String -> User
makeUser name = { name = name }

-- File: src/Main.gren
import User exposing (makeUser)
main = makeUser "Alice"
```

### Type Rename with Constructor
```gren
-- Before
type Status = Active | Inactive

processStatus : Status -> String
processStatus status =
    when status is
        Active -> "active"
        Inactive -> "inactive"

-- Rename: Status → UserStatus
-- After  
type UserStatus = Active | Inactive

processStatus : UserStatus -> String
processStatus status =
    when status is
        Active -> "active"
        Inactive -> "inactive"
```

### Module Rename
```gren
-- Before
-- File: src/Utils.gren
helper : String -> String

-- File: src/Main.gren
import Utils
main = Utils.helper "test"

-- Rename: Utils → Helpers (file rename)
-- After
-- File: src/Helpers.gren  
helper : String -> String

-- File: src/Main.gren
import Helpers  
main = Helpers.helper "test"
```

This story completes the professional refactoring capabilities by providing safe, accurate symbol renaming that maintains code integrity across large Gren projects, enabling confident refactoring essential for long-term codebase maintenance.