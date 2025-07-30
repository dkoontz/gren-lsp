# Tree-sitter AST Structure

## Overview

The Gren LSP implementation is **fundamentally based on tree-sitter AST parsing**. This document provides comprehensive guidance on the tree-sitter AST structure for Gren language constructs and establishes mandatory policies for symbol context determination.

## **CRITICAL POLICY: MANDATORY TREE-SITTER USAGE**

### ⚠️ **NEVER USE TEXT-BASED PARSING FOR SYMBOL CONTEXT**

**This is a hard requirement**: All symbol context determination MUST use tree-sitter AST analysis. This policy exists because:

1. **Information exists in the AST**: Every piece of context information about a symbol (location, type, scope, etc.) is accurately represented in the tree-sitter parse tree
2. **Text parsing is unreliable**: Regular expressions, string matching, and manual parsing lead to bugs and false positives
3. **AST is the single source of truth**: The tree-sitter parser provides the authoritative structure of Gren code

### Approved Use Cases for Text Processing

Text-based operations should ONLY be used for:
- **Display formatting**: Preparing content for user presentation
- **Content extraction**: Getting the actual text content from AST nodes
- **LSP protocol formatting**: Converting AST data to LSP protocol structures

### Common Anti-Patterns to Avoid

❌ **NEVER DO**: String matching to detect module declarations  
✅ **ALWAYS DO**: Query for `(module_declaration)` and `(exposing_list)` nodes

❌ **NEVER DO**: Regex patterns to find function definitions  
✅ **ALWAYS DO**: Query for `(value_declaration)` and `(function_declaration_left)` nodes  

❌ **NEVER DO**: Manual parentheses counting for export lists  
✅ **ALWAYS DO**: Navigate `(exposing_list)` → `(exposed_type)` / `(exposed_value)` nodes

❌ **NEVER DO**: Line-by-line parsing to detect comments  
✅ **ALWAYS DO**: Query for `(block_comment)` and `(line_comment)` nodes

## Tree-sitter AST Node Structure

### Top-Level File Structure

```
(file)
  ├── (module_declaration)
  ├── (import_clause)*
  ├── (block_comment | line_comment)*
  ├── (type_alias_declaration | type_declaration)*
  ├── (type_annotation)*
  └── (value_declaration)*
```

### Module Declaration Structure

```
(module_declaration)
  ├── (module) "module"
  ├── (upper_case_qid) "Module.Name"
  │   ├── (upper_case_identifier) "Module"
  │   ├── (dot) "."
  │   └── (upper_case_identifier) "Name"
  └── (exposing_list)
      ├── (exposing) "exposing"
      ├── (() "("
      ├── (exposed_type) | (exposed_value) | (exposed_union_constructors)
      └── ()) ")"
```

**Key Node Types for Module Context:**
- `(module_declaration)`: Entire module declaration including exposing clause
- `(exposing_list)`: The complete export list 
- `(exposed_type)`: Individual exported types (e.g., `Tetromino`)
- `(exposed_value)`: Individual exported functions (e.g., `initTetromino`)
- `(exposed_union_constructors)`: Type constructors (e.g., `Type (..)`)

### Import Declaration Structure

```
(import_clause)
  ├── (import) "import"
  ├── (upper_case_qid) "Module.Name"
  └── (exposing_list)?
      ├── (exposing) "exposing"
      ├── (() "("
      ├── (exposed_type) | (exposed_value)*
      └── ()) ")"
```

### Type Declarations

#### Type Alias
```
(type_alias_declaration)
  ├── (type) "type"
  ├── (alias) "alias"
  ├── (upper_case_identifier) "TypeName"
  ├── (eq) "="
  └── (type_expression)
      └── (record_type) | (type_ref) | ...
```

#### Union Type
```
(type_declaration)
  ├── (type) "type"
  ├── (upper_case_identifier) "TypeName"
  ├── (eq) "="
  ├── (union_variant)*
  └── (|)*
```

### Function Declarations

#### Type Annotation
```
(type_annotation)
  ├── (lower_case_identifier) "functionName"
  ├── (colon) ":"
  └── (type_expression)
      ├── (type_ref)*
      └── (arrow)*
```

#### Value Declaration  
```
(value_declaration)
  ├── (function_declaration_left)
  │   ├── (lower_case_identifier) "functionName"
  │   └── (lower_pattern)*
  ├── (eq) "="
  └── (expression)
```

### Expression Structures

#### Function Calls
```
(function_call_expr)
  ├── (value_expr)
  │   └── (value_qid)
  │       ├── (upper_case_identifier) "Module"    # For qualified calls
  │       ├── (dot) "."
  │       └── (lower_case_identifier) "function"
  └── (value_expr)* # arguments
```

#### Field Access
```
(field_access_expr)
  ├── (value_expr)
  │   └── (value_qid)
  │       └── (lower_case_identifier) "record"
  ├── (dot) "."
  └── (lower_case_identifier) "field"
```

#### Record Construction
```
(record_expr)
  ├── ({) "{"
  ├── (field)*
  │   ├── (lower_case_identifier) "fieldName"
  │   ├── (eq) "="
  │   └── (expression)
  └── (}) "}"
```

## Practical Implementation Examples

### Example 1: Detecting Module Exports

**❌ Wrong (Text-based)**:
```rust
fn is_in_export_list(line: &str) -> bool {
    line.trim_start().starts_with("(") && line.contains("exposing")
}
```

**✅ Correct (Tree-sitter)**:
```rust
async fn is_position_in_module_exports(&self, uri: &Url, line: u32, character: u32) -> bool {
    let query = tree_sitter::Query::new(language, r#"
        (exposing_list) @exports
    "#)?;
    
    // Check if position falls within any exposing_list node
    for capture in query_matches {
        if position_within_node(capture.node, line, character) {
            return true;
        }
    }
    false
}
```

### Example 2: Finding Function Definitions

**❌ Wrong (Regex)**:
```rust
fn find_functions(text: &str) -> Vec<String> {
    let re = Regex::new(r"^([a-z][a-zA-Z0-9_]*)\s*:").unwrap();
    // This misses functions without type annotations!
}
```

**✅ Correct (Tree-sitter Query)**:
```rust
async fn find_function_definitions(&self, uri: &Url) -> Result<Vec<FunctionDef>> {
    let query = tree_sitter::Query::new(language, r#"
        (value_declaration
          (function_declaration_left
            (lower_case_identifier) @function.name)) @function.declaration
        
        (type_annotation
          (lower_case_identifier) @function.type_name) @function.annotation
    "#)?;
    
    // Process matches to build complete function information
}
```

### Example 3: Symbol Context Detection

**✅ Complete Context Analysis**:
```rust
async fn get_symbol_context(&self, uri: &Url, position: Position) -> SymbolContext {
    let query = tree_sitter::Query::new(language, r#"
        ; Module exports
        (exposing_list) @context.module_export
        
        ; Import statements  
        (import_clause) @context.import
        
        ; Type definitions
        (type_declaration) @context.type_definition
        (type_alias_declaration) @context.type_alias
        
        ; Function definitions
        (value_declaration) @context.function_definition
        (type_annotation) @context.type_annotation
        
        ; Function calls
        (function_call_expr) @context.function_call
        
        ; Field access
        (field_access_expr) @context.field_access
        
        ; Comments
        (block_comment) @context.comment
    "#)?;
    
    // Determine which context the position falls within
}
```

## Tree-sitter Query Best Practices

### 1. Use Precise Node Selection
```rust
// Good: Specific node types
r#"(type_declaration (upper_case_identifier) @type.name)"#

// Avoid: Overly broad matching
r#"(upper_case_identifier) @name"#  // Too generic
```

### 2. Leverage Node Hierarchy
```rust
// Good: Use parent-child relationships  
r#"(module_declaration (exposing_list (exposed_type) @export))"#

// Avoid: Ignoring structure
r#"(exposed_type) @export"#  // Missing context
```

### 3. Capture Multiple Related Nodes
```rust
r#"
(value_declaration
  (function_declaration_left (lower_case_identifier) @function.name) @function.params
  (eq)
  (_ ) @function.body) @function.declaration
"#
```

## Integration with LSP Features

### Find All References
- Query for all `(value_qid)` and `(upper_case_qid)` nodes matching the symbol
- Filter by context using parent node analysis
- Exclude nodes within `(module_declaration)`, `(import_clause)`, `(block_comment)`

### Go to Definition  
- Query for `(value_declaration)` or `(type_declaration)` nodes with matching identifiers
- Use node position information for accurate location

### Hover Information
- Combine `(type_annotation)` and `(value_declaration)` queries
- Extract documentation from preceding `(block_comment)` nodes

### Symbol Search
- Query all identifier nodes with their parent context
- Build symbol hierarchy from AST structure

## Performance Considerations

- **Cache parsed trees**: Parse trees are expensive to generate
- **Reuse queries**: Compile tree-sitter queries once and reuse
- **Incremental parsing**: Leverage tree-sitter's incremental parsing capabilities
- **Lazy evaluation**: Only parse files when needed for LSP operations

## Debugging AST Structure

Use the debug_ast tool to examine any Gren code:

```bash
cargo run --bin debug_ast -- path/to/file.gren
```

This outputs the complete tree-sitter AST structure for understanding node relationships and implementing queries.

## Enforcement

This policy is enforced through:
- **Code reviews**: All PRs must use tree-sitter for symbol context
- **Architecture reviews**: Text-based parsing will be rejected
- **Documentation**: This policy is referenced in all story implementations
- **Testing**: Tests must validate tree-sitter query correctness

Any violation of the tree-sitter mandate should be treated as a critical architectural error requiring immediate remediation.