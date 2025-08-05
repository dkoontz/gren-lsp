# Epic 7 Story 4: Proactive Import Completion

## 📋 User Story
**As a** Gren developer  
**I want** intelligent completion suggestions that automatically add import statements for unimported symbols  
**So that** I can use functions and types from any module without manually managing imports

## ✅ Acceptance Criteria
- [ ] Extend textDocument/completion to include symbols from unimported modules
- [ ] Provide multiple completion variants for each unimported symbol:
  - Exposed import variant: `import Module exposing (symbol)` + use `symbol`
  - Qualified import variant: `import Module` + use `Module.symbol`
- [ ] Generate proper TextEdit operations to add import statements at correct file locations
- [ ] Maintain existing completion functionality (local scope, already imported modules)
- [ ] Handle import statement positioning (after existing imports, proper formatting)
- [ ] Support both function and type completions with automatic imports
- [ ] Provide clear visual indication of import completion variants in completion menu
- [ ] Ensure completion performance remains within requirements despite expanded symbol set

## 🧪 Integration Test Requirements

### Test: Basic Unimported Symbol Completion
- [ ] Create project with module `Utils` containing function `helper`
- [ ] In different file, type `hel` without importing `Utils`
- [ ] Verify completion shows two variants:
  - `helper` (adds `import Utils exposing (helper)`)
  - `Utils.helper` (adds `import Utils`)
- [ ] Test that selecting completion adds import and completes symbol correctly

### Test: Import Statement Positioning
- [ ] Create file with existing imports at top
- [ ] Trigger import completion for new unimported symbol
- [ ] Verify new import statement is added after existing imports
- [ ] Test proper alphabetical ordering of import statements
- [ ] Validate import formatting matches project conventions

### Test: Exposed vs Qualified Import Variants
- [ ] Test completion for unimported function shows both variants
- [ ] Verify exposed import completion:
  - Adds `import Module exposing (symbol)` 
  - Completes with bare `symbol` name
- [ ] Verify qualified import completion:
  - Adds `import Module`
  - Completes with `Module.symbol`
- [ ] Test that existing qualified imports are reused correctly

### Test: Type Import Completion
- [ ] Create module with custom type `User`
- [ ] In different file, type `Use` for type annotation
- [ ] Verify completion shows type import variants
- [ ] Test that type imports work correctly in type annotations
- [ ] Validate type constructor completion after type import

### Test: Existing Import Handling
- [ ] Create file that already imports some functions from a module
- [ ] Request completion for different function from same module
- [ ] Verify completion adds to existing `exposing` list rather than creating new import
- [ ] Test that qualified imports are reused when appropriate

### Test: Import Conflict Resolution
- [ ] Create scenario with same symbol name in multiple modules
- [ ] Verify completion shows variants for each module
- [ ] Test clear disambiguation in completion menu (module names shown)
- [ ] Validate that conflicts are handled gracefully

### Test: Performance with Large Symbol Set
- [ ] Test completion performance with 50+ modules containing 500+ symbols
- [ ] Verify response time remains within requirements
- [ ] Test that unimported symbol indexing doesn't significantly impact memory
- [ ] Validate incremental symbol loading if needed for performance

### Test: LSP Protocol Compliance
- [ ] Validate completion items include proper TextEdit operations for imports
- [ ] Test that additionalTextEdits are used correctly for import statements
- [ ] Verify completion item kinds are correct for functions, types, etc.
- [ ] Test proper UTF-16 position encoding for import statement locations

## 🔧 Technical Implementation

### Symbol Index Extensions
- Extend existing symbol index to include symbols from all workspace modules
- Index symbols even from modules not currently imported
- Maintain metadata about symbol origin (module, exposure type)
- Optimize symbol lookup for completion performance

### Import Analysis Engine
- Analyze existing import statements in current file
- Determine optimal import strategy for new symbols
- Handle import statement formatting and positioning
- Manage import conflicts and disambiguation

### Completion Item Generation
- Generate multiple completion variants for unimported symbols
- Create appropriate TextEdit operations for symbol completion
- Generate additionalTextEdits for import statement insertion
- Provide clear visual indicators for import completion types

### Import Statement Management
- Generate syntactically correct import statements
- Handle exposed imports, qualified imports, and mixed scenarios
- Maintain proper import statement ordering and formatting
- Update existing import statements when appropriate

## ⚡ Performance Requirements
- Completion response time: < 150ms for 95% of requests (slightly higher due to expanded symbol set)
- Symbol indexing: Efficient indexing of all workspace symbols without blocking
- Memory usage: Reasonable memory overhead for expanded symbol index
- Incremental updates: Efficient updates when modules change

## ✅ Definition of Done
- textDocument/completion extended to include unimported symbols from workspace
- Multiple completion variants generated for each unimported symbol (exposed/qualified)
- Import statements automatically added at correct file locations with proper formatting
- Existing import functionality preserved and integration seamless
- Performance requirements met despite expanded symbol set
- Integration tests validate all completion scenarios and import handling
- Clear visual indication of import completion variants in VS Code completion menu

## 📁 Related Files
- `src/import_completion.rs` - Proactive import completion engine
- `src/completion.rs` - Extend existing completion engine
- `src/import_manager.rs` - Import statement analysis and generation
- `src/import_completion_integration_tests.rs` - Comprehensive test coverage
- Integration with existing `src/symbol_index.rs` for workspace-wide symbol access

## 🔗 Dependencies
- Epic 2 Story 2 completed (Existing code completion infrastructure)
- Epic 2 Story 1 completed (Symbol indexing for workspace-wide symbol access)
- Existing import statement parsing and formatting utilities
- Tree-sitter queries for import statement analysis

## 📊 Status
**⏳ PENDING** - Ready for implementation

## 🎯 Success Metrics
- **User Experience**: Seamless import management without manual import statement writing
- **Coverage**: All workspace symbols accessible through completion with automatic imports
- **Performance**: Completion remains responsive despite expanded symbol set
- **Accuracy**: 100% correct import statement generation and positioning

## 💡 Import Completion Examples

### Basic Function Import Completion
```gren
-- Module: Utils.gren
module Utils exposing (helper, processor)

helper : String -> String
processor : Array String -> Array String

-- Module: Main.gren (typing "hel|")
module Main exposing (main)

-- Completion Menu Shows:
-- ┌─────────────────────────────────────────────────┐
-- │ helper                                         │ ← Exposed import
-- │   String -> String                             │
-- │   📦 import Utils exposing (helper)            │
-- │─────────────────────────────────────────────────│
-- │ Utils.helper                                   │ ← Qualified import  
-- │   String -> String                             │
-- │   📦 import Utils                              │
-- └─────────────────────────────────────────────────┘
```

### Selecting Exposed Import Variant
```gren
-- Before completion:
module Main exposing (main)

main = hel

-- After selecting "helper" (exposed import):
module Main exposing (main)

import Utils exposing (helper)

main = helper
```

### Selecting Qualified Import Variant
```gren
-- Before completion:
module Main exposing (main)

main = Uti

-- After selecting "Utils.helper" (qualified import):
module Main exposing (main)

import Utils

main = Utils.helper
```

### Extending Existing Import
```gren
-- Before completion (already has some imports from Utils):
module Main exposing (main)

import Utils exposing (processor)

main = 
    let
        result = processor data
        cleaned = hel  -- typing "hel|"
    in
    cleaned

-- After selecting "helper" (extends existing import):
module Main exposing (main)

import Utils exposing (processor, helper)

main = 
    let
        result = processor data
        cleaned = helper
    in
    cleaned
```

### Type Import Completion
```gren
-- Module: Types.gren
module Types exposing (User, Config)

type alias User = { name : String, age : Int }

-- Module: Main.gren (typing "processUser : Use|")
module Main exposing (main)

processUser : Use

-- Completion Menu Shows:
-- ┌─────────────────────────────────────────────────┐
-- │ User                                           │ ← Type exposed import
-- │   type alias                                   │
-- │   📦 import Types exposing (User)              │
-- │─────────────────────────────────────────────────│
-- │ Types.User                                     │ ← Type qualified import
-- │   type alias                                   │
-- │   📦 import Types                              │
-- └─────────────────────────────────────────────────┘
```

### Multiple Module Disambiguation
```gren
-- Module: Utils.gren
helper : String -> String

-- Module: Helpers.gren  
helper : Int -> String

-- Module: Main.gren (typing "hel|")
-- Completion Menu Shows:
-- ┌─────────────────────────────────────────────────┐
-- │ helper                                         │
-- │   String -> String                             │
-- │   📦 import Utils exposing (helper)            │
-- │─────────────────────────────────────────────────│
-- │ helper                                         │
-- │   Int -> String                                │
-- │   📦 import Helpers exposing (helper)          │
-- │─────────────────────────────────────────────────│
-- │ Utils.helper                                   │
-- │   String -> String                             │
-- │   📦 import Utils                              │
-- │─────────────────────────────────────────────────│
-- │ Helpers.helper                                 │
-- │   Int -> String                                │
-- │   📦 import Helpers                            │
-- └─────────────────────────────────────────────────┘
```

This story completes the developer experience by eliminating the friction of manual import management, allowing developers to focus on writing code while the LSP handles the mechanical aspects of importing symbols from across the workspace.