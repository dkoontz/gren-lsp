# Epic 7 Story 2: Signature Help for Functions

## 📋 User Story
**As a** Gren developer  
**I want** real-time function signature information while typing function calls  
**So that** I can understand function parameters, types, and documentation without looking up definitions

## ✅ Acceptance Criteria
- [ ] Implement textDocument/signatureHelp LSP handler with trigger character support
- [ ] Show function signatures with parameter names, types, and documentation
- [ ] Highlight active parameter based on cursor position and comma parsing
- [ ] Support both local and imported function signatures
- [ ] Handle function signatures with clear, unambiguous display (Gren has no overloads)
- [ ] Integrate with existing symbol index for efficient signature lookup
- [ ] Handle nested function calls and complex expressions
- [ ] Provide signature help for type constructors and record constructors

## 🧪 Integration Test Requirements

### Test: Basic Function Signature Help
- [ ] Create function call with cursor inside parentheses
- [ ] Trigger signatureHelp request (via typing "(" or comma)
- [ ] Verify signature appears with correct function name and parameters
- [ ] Validate parameter types are displayed accurately
- [ ] Test that documentation is included when available

### Test: Active Parameter Highlighting
- [ ] Position cursor on first parameter of function call
- [ ] Verify first parameter is highlighted as active
- [ ] Move cursor past comma to second parameter
- [ ] Verify second parameter becomes active
- [ ] Test active parameter tracking with nested parentheses

### Test: Cross-Module Function Signatures
- [ ] Create function call to imported function from another module
- [ ] Verify signature help shows correct imported function signature
- [ ] Test qualified function calls (Module.function)
- [ ] Validate unqualified imports work correctly
- [ ] Test aliased module imports

### Test: Type Constructor Signatures
- [ ] Create type constructor call (e.g., Just value, User name email)
- [ ] Verify signature help shows constructor parameters
- [ ] Test record constructor patterns
- [ ] Validate union type constructor signatures

### Test: Complex Expression Handling
- [ ] Test signature help within nested function calls
- [ ] Verify signature help in let expressions
- [ ] Test function calls within record expressions
- [ ] Validate signature help in pattern matching contexts

### Test: Performance and Responsiveness
- [ ] Test signature help response time meets requirements
- [ ] Verify no blocking during signature resolution
- [ ] Test behavior with large files and many functions
- [ ] Validate memory usage during intensive signature operations

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC response format matches LSP 3.18 spec
- [ ] Test SignatureHelp structure with proper signatures array
- [ ] Verify active signature and parameter indices
- [ ] Test proper UTF-16 position encoding for signatures

## 🔧 Technical Implementation

### Signature Resolution Engine
- Leverage existing symbol index for function lookup
- Parse function type signatures from symbol information
- Extract parameter names, types, and documentation
- Handle qualified vs unqualified function references

### Active Parameter Detection
- Parse expression context around cursor position
- Count commas and parentheses to determine active parameter
- Handle nested expressions and complex call sites
- Support various Gren function call patterns

### LSP Handler Implementation
- Implement `textDocument/signatureHelp` message handler
- Configure trigger characters ("(", ",") for automatic invocation
- Generate SignatureInformation with proper parameter details
- Handle single signature per function (Gren's deterministic nature)

### Documentation Integration
- Extract function documentation from symbol index
- Format documentation for display in signature help
- Include type information and parameter descriptions
- Support markdown formatting in documentation

## ⚡ Performance Requirements
- Response time: < 100ms for 95% of signature help requests
- Support files with 100+ function definitions efficiently
- Minimal memory overhead for signature caching
- Fast symbol resolution for immediate user feedback

## ✅ Definition of Done
- textDocument/signatureHelp handler implemented and tested
- Function signatures display accurately with parameter information
- Active parameter highlighting works correctly during typing
- Cross-module function signatures resolve properly
- Integration tests validate signature accuracy and performance
- VS Code integration provides seamless signature help experience
- Error handling for invalid or unresolved function calls

## 📁 Related Files
- `src/signature_help.rs` - Main SignatureHelpEngine implementation
- `src/lsp_service.rs` - LSP handler integration and capability advertisement
- `src/signature_help_integration_tests.rs` - Comprehensive test coverage
- Integration with existing `src/symbol_index.rs` for function lookup
- Extensions to existing `src/expression_parser.rs` for context analysis

## 🔗 Dependencies
- Epic 2 completed (Symbol indexing for function signatures)
- Existing symbol resolution and type information
- Expression parsing utilities for cursor context
- LSP message handling framework

## 📊 Status
**⏳ PENDING** - Ready for implementation

## 🎯 Success Metrics
- **Accuracy**: 100% correct signature information for resolvable functions (single signature per function)
- **Performance**: Sub-100ms response time for signature lookup
- **User Experience**: Immediate, helpful signature information during typing
- **Coverage**: Support for all Gren function types and call patterns

## 💡 Signature Help Examples

### Local Function Signature
```gren
-- Function definition
createUser : String -> Int -> User
createUser name age = { name = name, age = age }

-- While typing: createUser("Alice", |)
-- Signature Help Shows:
-- createUser(name: String, age: Int) -> User
--                          ^parameter 2 active (cursor after comma)
```

### Imported Function Signature
```gren
import String

-- While typing: String.contains("test", |)
-- Signature Help Shows:
-- String.contains(substring: String, string: String) -> Bool
--                                    ^parameter 2 active
```

### Type Constructor Signature
```gren
type User = User String Int

-- While typing: User "Alice" |
-- Signature Help Shows:
-- User(name: String, age: Int) -> User
--                    ^parameter 2 active
```

### Record Constructor Signature
```gren
type alias DatabaseConfig = 
    { host : String
    , port : Int  
    , database : String
    , username : String
    , timeout : Int
    }

-- While typing: { host = "localhost", timeout = 5000, |
-- Signature Help Shows:
-- DatabaseConfig { host: String, port: Int, database: String, username: String, timeout: Int }
--                                ^port active (next unspecified field)
-- 
-- Note: Fields can be provided in any order. Signature help shows the next unspecified field,
-- in this case 'port' since 'host' and 'timeout' are already provided.
```

### Function with Documentation
```gren
{-| Processes user input by trimming whitespace and converting to uppercase.

Parameters:
- input: The string to process
- options: Processing configuration

Returns the processed string.
-}
processInput : String -> ProcessOptions -> String

-- While typing: processInput("test", |)
-- Signature Help Shows:
-- processInput(input: String, options: ProcessOptions) -> String
-- 
-- Processes user input by trimming whitespace and converting to uppercase.
-- 
-- Parameters:
-- - input: The string to process  
-- - options: Processing configuration
--                             ^parameter 2 active
```

### Nested Function Call
```gren
-- While typing: Array.map (String.toUpper |) items
-- Signature Help Shows:
-- String.toUpper(string: String) -> String
--                ^parameter 1 active
```

This story enhances developer productivity by providing immediate, contextual function information that eliminates the need to constantly reference documentation or function definitions, creating a smooth and efficient coding experience.