# Stories

## Story 2.1: Integrate tree-sitter-gren Parser
**Description:** Set up tree-sitter with the Gren grammar for incremental parsing of source files.

**Acceptance Criteria:**
- [x] Add tree-sitter and tree-sitter-gren dependencies
- [x] Create parser initialization with Gren grammar
- [x] Implement parse_file function returning tree-sitter AST
- [x] Handle parse errors gracefully
- [x] Set up incremental parsing for file changes
- [x] Add performance benchmarks for parsing

**Technical Notes:**
- May need to contribute to tree-sitter-gren for missing features
- Cache parsed trees for performance
- Handle malformed syntax without crashing

## Story 2.2: Implement Document Synchronization
**Description:** Handle textDocument synchronization events to maintain accurate file state.

**Acceptance Criteria:**
- [x] Handle textDocument/didOpen notifications
- [x] Process textDocument/didChange with incremental updates
- [x] Implement textDocument/didClose cleanup
- [x] Manage document versions correctly
- [x] Integrate with tree-sitter incremental parsing
- [x] Maintain document cache with LRU eviction

**Technical Notes:**
- Use lsp-textdocument for version management
- Ensure thread-safe document access
- Optimize for rapid typing scenarios

## Story 2.3: Create Basic Syntax Error Diagnostics
**Description:** Report syntax errors from tree-sitter parsing as LSP diagnostics.

**Acceptance Criteria:**
- [x] Extract error nodes from tree-sitter parse tree
- [x] Convert parse errors to LSP Diagnostic format
- [x] Publish diagnostics via textDocument/publishDiagnostics
- [x] Clear diagnostics when errors are fixed
- [x] Include helpful error messages
- [x] Report error ranges accurately

**Technical Notes:**
- Batch diagnostic updates for performance
- Provide Gren-specific error messages
- Consider error recovery strategies

## Story 2.4: Implement Symbol Extraction
**Description:** Extract symbols (functions, types, modules) from parsed AST for indexing.

**Acceptance Criteria:**
- [x] Create tree-sitter queries for Gren symbols
- [x] Extract function definitions with signatures
- [x] Identify type definitions and aliases
- [x] Capture module declarations and imports
- [x] Store symbols in SQLite database
- [x] Update symbols on file changes

**Technical Notes:**
- Design efficient tree-sitter query patterns
- Handle nested symbols correctly
- Maintain symbol relationships

## Story 2.4.1: Implement LSP Symbol Protocol Methods
**Description:** Expose extracted symbols to LSP clients through standard protocol methods.

**Acceptance Criteria:**
- [x] Implement textDocument/documentSymbol for file outline view
- [x] Implement workspace/symbol for global symbol search
- [x] Return hierarchical symbols with proper nesting
- [x] Include symbol kinds, ranges, and selection ranges
- [x] Support symbol filtering and search queries
- [x] Handle symbol icons and descriptions in VS Code

**Technical Notes:**
- Convert internal Symbol format to LSP DocumentSymbol/SymbolInformation
- Implement symbol hierarchy for nested types and modules
- Optimize for VS Code Outline panel and Ctrl+T search
- Consider symbol caching for performance

## Story 2.5: Enable Basic Code Completion
**Description:** Provide code completion for symbols in the current file and imported modules.

**Acceptance Criteria:**
- [x] Handle textDocument/completion requests
- [x] Complete local symbols from current file
- [x] Include imported symbols from other modules
- [x] Provide completion item details (type, docs)
- [x] Support keyword completion
- [x] Filter results based on context

**Technical Notes:**
- Implement fuzzy matching for better UX
- Cache completion results when possible
- Consider completion performance targets

## Story 2.6: Implement Go-to-Definition
**Description:** Enable navigation to symbol definitions within the workspace.

**Acceptance Criteria:**
- [ ] Handle textDocument/definition requests
- [ ] Resolve symbols at cursor position
- [ ] Find definition locations in workspace
- [ ] Support cross-file navigation
- [ ] Handle module imports correctly
- [ ] Provide fallback for external dependencies

**Technical Notes:**
- Use symbol index for fast lookups
- Handle multiple definition candidates
- Support peek definition if possible

## Story 2.7: Add Hover Information
**Description:** Display type signatures and documentation on hover.

**Acceptance Criteria:**
- [x] Handle textDocument/hover requests
- [x] Show type signatures for functions
- [x] Display documentation if available
- [x] Format hover content as Markdown
- [x] Include module information
- [x] Respond quickly to hover requests

**Technical Notes:**
- Extract doc comments from source
- Cache hover information
- Keep hover content concise

## Story 2.8: Create Workspace Symbol Index
**Description:** Build and maintain a searchable index of all symbols in the workspace.

**Acceptance Criteria:**
- [x] Index all Gren files on workspace initialization
- [x] Update index on file changes
- [x] Store symbols with metadata in SQLite
- [x] Implement efficient symbol search queries
- [x] Handle workspace folder changes
- [x] Provide progress reporting for indexing

**Technical Notes:**
- Use parallel processing for initial indexing
- Implement incremental index updates
- Design for large workspace scalability

## Story 2.9: Extract Documentation Comments
**Description:** Parse and extract Gren documentation comments ({-| ... -}) and associate them with symbols.

**Acceptance Criteria:**
- [x] Parse Gren doc comments ({-| ... -}) from source code
- [x] Associate doc comments with their corresponding symbols during extraction
- [x] Store documentation in symbol database
- [x] Update hover to display extracted documentation
- [x] Clean and format documentation text appropriately

**Technical Notes:**
- Documentation comments use {-| ... -} syntax in Gren
- Comments should be associated with symbols that appear 1-3 lines after the comment ends
- Clean documentation by removing excess whitespace and empty lines
- Display documentation in hover alongside type signatures and symbol information
