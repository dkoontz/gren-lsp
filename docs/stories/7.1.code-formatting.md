# Epic 7 Story 1: Code Formatting Integration

## 📋 User Story
**As a** Gren developer  
**I want** consistent code formatting integrated with my editor  
**So that** I can maintain clean, readable code without manual formatting work

## ✅ Acceptance Criteria
- [ ] Implement textDocument/formatting LSP handler for full document formatting
- [ ] Implement textDocument/rangeFormatting LSP handler for selected text formatting
- [ ] Integrate with Gren compiler's official formatter for consistency
- [ ] Ensure formatted code maintains syntactic correctness (100% compilation success)
- [ ] Support editor "Format Document" and "Format Selection" commands
- [ ] Preserve semantic meaning while improving code style
- [ ] Handle formatting errors gracefully with fallback behavior
- [ ] Support configuration for formatting preferences where applicable

## 🧪 Integration Test Requirements

### Test: Full Document Formatting
- [ ] Create Gren file with inconsistent formatting (mixed indentation, spacing)
- [ ] Apply textDocument/formatting request
- [ ] Verify output matches official Gren formatter style
- [ ] Validate formatted code compiles successfully

### Test: Range Formatting
- [ ] Create Gren file with mixed formatting throughout
- [ ] Select specific function or type definition
- [ ] Apply textDocument/rangeFormatting request
- [ ] Verify only selected range is formatted, rest unchanged
- [ ] Validate partial formatting maintains file syntax

### Test: Syntax Preservation
- [ ] Format file with complex Gren constructs (nested functions, pattern matching)
- [ ] Verify all language constructs are preserved correctly
- [ ] Test that semantic meaning is unchanged after formatting
- [ ] Validate type annotations and signatures remain intact

### Test: Error Handling
- [ ] Attempt to format file with syntax errors
- [ ] Verify graceful error handling without crashing
- [ ] Test fallback behavior when formatter is unavailable
- [ ] Validate appropriate error messages for formatting failures

### Test: Large File Performance
- [ ] Format large Gren files (1000+ lines)
- [ ] Verify formatting completes within performance requirements
- [ ] Test memory usage remains bounded during formatting
- [ ] Validate incremental formatting for selected ranges

### Test: Editor Integration
- [ ] Test VS Code "Format Document" command integration
- [ ] Verify "Format Selection" works with range formatting
- [ ] Test format-on-save functionality if supported
- [ ] Validate formatting works across different LSP clients

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC response format matches LSP 3.18 spec
- [ ] Test TextEdit array structure with proper ranges
- [ ] Verify UTF-16 position encoding for formatted text
- [ ] Test proper error responses for formatting failures

## 🔧 Technical Implementation

### Gren Compiler Integration
- Use official Gren formatter via compiler integration
- Invoke formatter on temporary files for safety
- Parse formatter output and convert to LSP TextEdit operations
- Handle formatter-specific command line arguments and options

### LSP Handler Implementation
- Implement `textDocument/formatting` message handler
- Implement `textDocument/rangeFormatting` message handler
- Generate appropriate TextEdit operations for formatting changes
- Support configurable formatting options through LSP settings

### Text Edit Generation
- Calculate precise text ranges for formatting changes
- Generate minimal TextEdit operations to reduce editor disruption
- Handle multi-line changes and whitespace modifications
- Ensure non-overlapping edit ranges for proper application

### Error Recovery and Fallback
- Graceful handling when Gren formatter is unavailable
- Fallback behavior for files with syntax errors
- Clear error reporting for formatting failures
- Maintain document stability during formatting errors

## ⚡ Performance Requirements
- Full document formatting: < 500ms for files up to 1000 lines
- Range formatting: < 200ms for typical selections
- Memory usage: Efficient handling without memory leaks
- Support concurrent formatting requests safely

## ✅ Definition of Done
- textDocument/formatting and rangeFormatting handlers implemented and tested
- Gren compiler formatter integration produces consistent, high-quality output
- Formatted code maintains 100% syntactic correctness with compiler validation
- Performance requirements met for typical development scenarios
- Integration tests validate formatting accuracy and editor compatibility
- Error handling provides graceful degradation and clear user feedback
- VS Code extension integration works seamlessly with editor formatting commands

## 📁 Related Files
- `src/code_formatting.rs` - Main CodeFormattingEngine implementation
- `src/lsp_service.rs` - LSP handler integration and capability advertisement
- `src/code_formatting_integration_tests.rs` - Comprehensive test coverage
- Integration with existing `src/compiler_integration.rs` for formatter access
- Extensions to existing `src/text_edit_utils.rs` for edit generation

## 🔗 Dependencies
- Epic 1-2 completed (LSP foundation, compiler integration)
- Existing Gren compiler integration for formatter access
- Text editing utilities for TextEdit generation
- LSP message handling framework

## 📊 Status
**⏳ PENDING** - Ready for implementation

## 🎯 Success Metrics
- **Code Quality**: 100% syntactically correct formatted output
- **Performance**: Sub-500ms response time for document formatting
- **User Experience**: Seamless integration with editor formatting commands
- **Reliability**: Zero formatting operations that break compilation

## 💡 Formatting Examples

### Before Formatting
```gren
module Utils exposing(helper,process)
import Array
import String

type alias Config={timeout:Int,retries:Int}

helper:String->String
helper input=
  let
    trimmed=String.trim input
    processed=String.toUpper trimmed
  in
    processed

process:Array String->Array String
process items=Array.map(\item->
  helper item
  ) items
```

### After Formatting
```gren
module Utils exposing (helper, process)

import Array
import String


type alias Config =
    { timeout : Int
    , retries : Int
    }


helper : String -> String
helper input =
    let
        trimmed =
            String.trim input

        processed =
            String.toUpper trimmed
    in
    processed


process : Array String -> Array String
process items =
    Array.map
        (\item ->
            helper item
        )
        items
```

### Range Formatting Example
```gren
-- Before (selecting only the helper function)
helper:String->String
helper input=
  let
    trimmed=String.trim input
    processed=String.toUpper trimmed
  in
    processed

-- After (only selected function formatted)
helper : String -> String
helper input =
    let
        trimmed =
            String.trim input

        processed =
            String.toUpper trimmed
    in
    processed
```

This story provides essential code formatting capabilities that complete the professional development experience, ensuring consistent code style across Gren projects while maintaining the reliability and performance standards established in previous epics.