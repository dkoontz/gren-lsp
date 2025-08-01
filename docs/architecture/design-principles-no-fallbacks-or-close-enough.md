# Design Principles: No Fallbacks or "Close Enough"

The Gren LSP server follows strict precision principles that align with Gren's deterministic language characteristics:

## Exact Matching Requirements
- **Test Assertions**: All test cases must match literal strings and exact values, no fuzzy matching or approximations
- **Symbol Resolution**: Every symbol must have exactly one definition determinable from the tree-sitter AST
- **Position Calculations**: UTF-16 positions must be calculated exactly as per LSP specification
- **Compiler Integration**: Use only the exact compiler path specified, never assume PATH or alternative versions

## No Approximation Examples

**❌ Incorrect Approaches**:
```rust
// DON'T: Fuzzy string matching in tests
assert!(response.contains("Html"));  // Too permissive

// DON'T: Fallback compiler detection
let compiler = find_gren_compiler()  // Searches multiple locations
    .or_else(|| try_path_compiler())
    .unwrap_or_else(|| default_compiler());

// DON'T: Approximate symbol resolution
if exact_match.is_none() {
    return find_similar_symbols(name);  // "Close enough" matching
}
```

**✅ Correct Approaches**:
```rust
// DO: Exact string matching in tests
assert_eq!(response.label, "text");  // Precise expectation

// DO: Exact compiler path requirement
let compiler_path = env::var("GREN_COMPILER_PATH")
    .map_err(|_| Error::CompilerPathNotSet)?;
if !Path::new(&compiler_path).exists() {
    return Err(Error::CompilerNotFound(compiler_path));
}

// DO: Deterministic symbol resolution
let definition = resolve_symbol_from_ast(&symbol_name, &tree)
    .ok_or(Error::SymbolNotFound(symbol_name))?;
return Ok(vec![definition]);  // Exactly one result or error
```

## Deterministic Behavior
- **Symbol Resolution**: Leverage Gren's lack of polymorphic overloading for precise, single-result symbol lookups
- **Import Resolution**: Use explicit import statements to determine exact symbol sources
- **Error Reporting**: Provide exact error locations and specific remediation steps
- **Type Information**: Display precise type signatures without approximation

## Configuration Strictness
- **Environment Variables**: Required variables must be explicitly set, no defaults assumed
- **Project Structure**: Validate exact project structure requirements (gren.json, src/ directory)
- **Dependency Paths**: Use exact package paths from gren.json, no path resolution heuristics

This strict approach eliminates ambiguity and ensures reliable, predictable behavior that developers can depend on, matching Gren's philosophy of eliminating runtime surprises through compile-time precision.
