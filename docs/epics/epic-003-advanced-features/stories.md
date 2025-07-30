# Stories

## Story 3.1: Gren Compiler Integration Layer
**Description:** Create an integration layer to communicate with the Gren compiler for type checking and advanced analysis.

**Acceptance Criteria:**
- [x] Design compiler communication protocol (CLI or FFI)
- [x] Implement compiler process management
- [x] Parse compiler output for diagnostics
- [x] Handle compiler errors gracefully
- [x] Cache compilation results
- [x] Support incremental compilation where possible

**Technical Notes:**
- Start with CLI integration, plan for future FFI
- Handle compiler version compatibility
- Manage compiler process lifecycle

## Story 3.2: Type-based Diagnostics
**Description:** Report type errors and warnings from the Gren compiler as LSP diagnostics.

**Acceptance Criteria:**
- [ ] Run type checking on file save
- [ ] Parse type errors from compiler output
- [ ] Convert to LSP diagnostic format with proper ranges
- [ ] Distinguish between errors and warnings
- [ ] Provide helpful error messages with hints
- [ ] Support real-time type checking (debounced)

**Technical Notes:**
- Merge with syntax diagnostics appropriately
- Cache type checking results
- Optimize for incremental checking

## Story 3.3: Implement Rename Refactoring
**Description:** Enable safe renaming of symbols across the entire workspace.

**Acceptance Criteria:**
- [ ] Handle textDocument/rename requests
- [ ] Find all references to the symbol
- [ ] Validate new name is valid Gren identifier
- [ ] Generate workspace edit for all occurrences
- [ ] Handle module-qualified names correctly
- [ ] Preview changes before applying

**Technical Notes:**
- Ensure atomicity of rename operation
- Handle edge cases like shadowing
- Test with large refactorings

## Story 3.4: Find All References
**Description:** Locate all usages of a symbol throughout the workspace.

**Acceptance Criteria:**
- [ ] Handle textDocument/references requests
- [ ] Search all files for symbol usage
- [ ] Include direct and qualified references
- [ ] Distinguish declarations from references
- [ ] Support filtering by reference type
- [ ] Optimize search performance

**Technical Notes:**
- Leverage symbol index for efficiency
- Handle renamed imports correctly
- Support incremental index updates

## Story 3.5: Import Management
**Description:** Provide intelligent import suggestions and organization.

**Acceptance Criteria:**
- [ ] Suggest imports for unresolved symbols
- [ ] Generate import statements automatically
- [ ] Remove unused imports on request
- [ ] Organize imports alphabetically
- [ ] Handle exposing lists correctly
- [ ] Support quick fix code actions

**Technical Notes:**
- Understand Gren module system deeply
- Cache available exports per module
- Respect existing import style

## Story 3.6: Code Formatting Integration
**Description:** Integrate with Gren formatter (when available) or implement basic formatting.

**Acceptance Criteria:**
- [ ] Handle textDocument/formatting requests
- [ ] Format entire documents on request
- [ ] Support range formatting
- [ ] Preserve semantic meaning
- [ ] Configure formatting options
- [ ] Integrate with format-on-save

**Technical Notes:**
- May need to implement basic formatter initially
- Ensure idempotent formatting
- Handle malformed code gracefully

## Story 3.7: Code Actions and Quick Fixes
**Description:** Provide contextual code actions for common tasks and error fixes.

**Acceptance Criteria:**
- [ ] Handle textDocument/codeAction requests
- [ ] Suggest fixes for common type errors
- [ ] Add missing type annotations
- [ ] Convert between related types
- [ ] Extract expressions to functions
- [ ] Generate case expressions for custom types

**Technical Notes:**
- Priority on most useful actions
- Ensure generated code is idiomatic
- Test action application thoroughly

## Story 3.8: Document Symbols and Outline
**Description:** Provide document structure for navigation and outline views.

**Acceptance Criteria:**
- [ ] Handle textDocument/documentSymbol requests
- [ ] Return hierarchical symbol structure
- [ ] Include all functions, types, and values
- [ ] Support symbol kinds appropriately
- [ ] Enable breadcrumb navigation
- [ ] Update dynamically with changes

**Technical Notes:**
- Optimize for UI responsiveness
- Handle nested definitions correctly
- Provide meaningful symbol ranges
