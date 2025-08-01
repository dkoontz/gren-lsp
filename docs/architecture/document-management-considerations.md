# Document Management Considerations

## UTF-16 Position Encoding
The lsp-textdocument crate **only supports UTF-16** position encoding, which aligns perfectly with the LSP specification requirement. This eliminates a common source of bugs in language server implementations:

- **LSP Standard**: LSP uses UTF-16 code units for position calculations by default
- **Client Compatibility**: Ensures compatibility with VS Code and other major editors
- **Position Accuracy**: Eliminates off-by-one errors common with naive UTF-8 calculations
- **Unicode Handling**: Correctly handles multi-byte Unicode characters and surrogate pairs

## Integration Trade-offs

**Benefits**:
- **Proven Algorithms**: Uses VS Code's text document implementation as foundation
- **Reduced Complexity**: Eliminates need to implement incremental text synchronization
- **Bug Prevention**: Avoids common position calculation and document state bugs
- **Maintenance**: Delegates complex edge cases to a specialized library

**Limitations**:
- **UTF-16 Only**: Cannot easily switch to UTF-8 or UTF-32 position encoding if needed
- **External Dependency**: Adds another crate dependency to the project
- **API Constraints**: Must work within the lsp-textdocument API design

**Mitigation Strategies**:
- **Hybrid Approach**: Use lsp-textdocument for document lifecycle and position mapping, but maintain direct content access for tree-sitter parsing
- **Fallback Option**: Keep the ability to implement custom document management if lsp-textdocument limitations become problematic
- **Performance Monitoring**: Monitor memory usage and performance impact of the additional abstraction layer

## Alternative Considerations

If lsp-textdocument proves unsuitable, alternative approaches include:
- **rope-science**: More advanced rope data structure for large documents
- **ropey**: Another rope implementation optimized for text editing
- **Custom Implementation**: Direct implementation using lsp-types for maximum control

However, lsp-textdocument's proven track record and LSP-specific optimizations make it the recommended choice for this implementation.
