# Epic 7 Story 3: Advanced Navigation Aids

## 📋 User Story
**As a** Gren developer  
**I want** intelligent text selection and code folding capabilities  
**So that** I can efficiently navigate and manipulate large files with complex code structures

## ✅ Acceptance Criteria
- [ ] Implement textDocument/selectionRange LSP handler for intelligent text selection
- [ ] Implement textDocument/foldingRange LSP handler for code folding
- [ ] Support hierarchical selection ranges following Gren syntax boundaries
- [ ] Provide logical folding ranges for functions, types, modules, and expressions
- [ ] Handle nested structures and complex Gren language constructs
- [ ] Integrate with editor selection expansion and code folding UI
- [ ] Support both line-based and character-based folding ranges
- [ ] Optimize for large files with efficient range calculation

## ✅ Acceptance Criteria

### Selection Range Requirements
- [ ] Support progressive selection expansion (symbol → expression → statement → block)
- [ ] Handle Gren-specific constructs (let expressions, when branches, function definitions)
- [ ] Provide accurate selection boundaries for all syntax elements
- [ ] Support nested selection ranges with proper hierarchy

### Folding Range Requirements
- [ ] Enable folding of function definitions with proper boundaries
- [ ] Support type definition folding (union types, type aliases, records)
- [ ] Allow folding of import sections and module declarations
- [ ] Provide folding for let expressions and complex nested structures
- [ ] Support comment block folding for documentation

## 🧪 Integration Test Requirements

### Test: Progressive Selection Expansion
- [ ] Position cursor on variable name within expression
- [ ] Request selectionRange repeatedly to expand selection
- [ ] Verify progression: variable → expression → statement → function → module
- [ ] Test selection boundaries align with Gren syntax structure
- [ ] Validate nested expression selection works correctly

### Test: Gren Language Construct Selection
- [ ] Test selection ranges for let expressions (let → in boundaries)
- [ ] Verify when expression selection (branches, pattern matching)
- [ ] Test function definition selection (signature + body)
- [ ] Validate type definition selection (constructors, fields)
- [ ] Test record expression and update selection

### Test: Function Definition Folding
- [ ] Create functions with type signatures and implementations
- [ ] Request foldingRange for file with multiple functions
- [ ] Verify each function can be folded independently
- [ ] Test that folded functions show signature or summary
- [ ] Validate folding boundaries don't interfere with syntax

### Test: Type Definition Folding
- [ ] Create union types with multiple constructors
- [ ] Test folding of type alias definitions
- [ ] Verify record type folding with multiple fields
- [ ] Test nested type definition folding
- [ ] Validate folding preserves type signature visibility

### Test: Module Structure Folding
- [ ] Test import section folding (multiple imports)
- [ ] Verify module declaration folding if applicable
- [ ] Test documentation comment folding
- [ ] Validate nested module structure folding
- [ ] Test that essential declarations remain visible when folded

### Test: Complex Expression Folding
- [ ] Test let expression folding (let bindings → in expression)
- [ ] Verify when expression folding (branches)
- [ ] Test nested function call folding
- [ ] Validate list/array literal folding
- [ ] Test record expression folding

### Test: Performance with Large Files
- [ ] Test selection range calculation on 1000+ line files
- [ ] Verify folding range generation meets performance requirements
- [ ] Test that range calculation doesn't block editor interaction
- [ ] Validate memory usage during intensive range operations

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC response format matches LSP 3.18 spec
- [ ] Test SelectionRange structure with proper parent/child relationships
- [ ] Verify FoldingRange structure with start/end lines and characters
- [ ] Test proper UTF-16 position encoding for all ranges

## 🔧 Technical Implementation

### Selection Range Engine
- Use tree-sitter AST for accurate syntax boundary detection
- Implement hierarchical selection range calculation
- Handle Gren-specific language constructs and expressions
- Generate parent-child selection range relationships

### Folding Range Engine
- Analyze tree-sitter parse tree for logical folding boundaries
- Generate line-based and character-based folding ranges
- Handle nested structures and overlapping ranges
- Optimize folding range calculation for large files

### Tree-sitter Query Extensions
- Create advanced tree-sitter queries for selection boundaries
- Implement queries for folding range detection
- Handle complex nested expressions and statements
- Support incremental range updates for performance

### LSP Handler Implementation
- Implement `textDocument/selectionRange` message handler
- Implement `textDocument/foldingRange` message handler
- Generate appropriate range structures for LSP responses
- Support efficient range caching and incremental updates

## ⚡ Performance Requirements
- Selection range calculation: < 200ms for typical cursor positions
- Folding range generation: < 300ms for files up to 1000 lines
- Memory usage: Efficient range storage without memory leaks
- Support concurrent range requests safely

## ✅ Definition of Done
- textDocument/selectionRange and foldingRange handlers implemented and tested
- Selection expansion follows Gren syntax boundaries accurately
- Code folding provides logical folding points for all major constructs
- Performance requirements met for large file navigation
- Integration tests validate range accuracy and editor compatibility
- VS Code integration works seamlessly with editor selection and folding UI
- Error handling provides graceful degradation for parsing issues

## 📁 Related Files
- `src/selection_range.rs` - SelectionRangeEngine implementation
- `src/folding_range.rs` - FoldingRangeEngine implementation
- `src/lsp_service.rs` - LSP handler integration and capability advertisement
- `src/navigation_integration_tests.rs` - Comprehensive test coverage
- Extensions to existing `src/tree_sitter_queries.rs` for advanced queries
- Integration with existing `src/document_management.rs` for file parsing

## 🔗 Dependencies
- Epic 1 completed (Tree-sitter parsing infrastructure)
- Existing document management and parsing systems
- Tree-sitter query system for syntax analysis
- LSP message handling framework

## 📊 Status
**⏳ PENDING** - Ready for implementation

## 🎯 Success Metrics
- **Navigation Efficiency**: Intuitive selection expansion following language structure
- **Code Organization**: Logical folding that improves large file readability
- **Performance**: Sub-300ms response time for range calculations
- **User Experience**: Seamless integration with editor navigation features

## 💡 Navigation Examples

### Progressive Selection Expansion
```gren
-- Starting with cursor on "name" in: user.name
-- Selection 1: |name|
-- Selection 2: |user.name|
-- Selection 3: |String.toUpper user.name|
-- Selection 4: |email = String.toUpper user.name|
-- Selection 5: |{ user | email = String.toUpper user.name }|

updateUser : User -> User
updateUser user =
    { user | email = String.toUpper user.name }
```

### Function Definition Folding
```gren
-- Unfolded
createUser : String -> Int -> User
createUser name age =
    let
        trimmedName =
            String.trim name
        
        validAge =
            max 0 age
    in
    { name = trimmedName, age = validAge }

-- Folded (shows signature, hides implementation)
createUser : String -> Int -> User ⏷
```

### Type Definition Folding
```gren
-- Unfolded
type User
    = Anonymous
    | Registered String Int
    | Premium String Int (Array String)

-- Folded
type User ⏷
```

### Let Expression Folding
```gren
-- Unfolded
processUser user =
    let
        normalizedName =
            String.trim user.name
            |> String.toLower
        
        validatedAge =
            if user.age < 0 then
                0
            else
                user.age
        
        updatedUser =
            { user 
            | name = normalizedName
            , age = validatedAge
            }
    in
    updatedUser

-- Folded
processUser user =
    let ⏷
    in
    updatedUser
```

### When Expression Folding
```gren
-- Unfolded
handleMessage msg model =
    when msg is
        UserClicked userId ->
            ( { model | selectedUser = Just userId }
            , Cmd.none
            )
        
        UserDeleted userId ->
            ( { model 
              | users = Array.filter (\u -> u.id /= userId) model.users
              , selectedUser = Nothing
              }
            , Cmd.none
            )
        
        NoOp ->
            ( model, Cmd.none )

-- Folded
handleMessage msg model =
    when msg is ⏷
```

### Import Section Folding
```gren
-- Unfolded
import Array
import String
import Html exposing (Html, div, text, button)
import Html.Attributes exposing (class, id)
import Html.Events exposing (onClick)
import Json.Decode as Decode
import Json.Encode as Encode

-- Folded
import Array ⏷
```

This story completes the essential navigation aids that make working with large Gren files efficient and enjoyable, providing the final pieces needed for a professional development experience that rivals commercial IDEs.