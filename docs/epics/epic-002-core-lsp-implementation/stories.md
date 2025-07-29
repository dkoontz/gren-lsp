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
- [ ] Handle textDocument/didOpen notifications
- [ ] Process textDocument/didChange with incremental updates
- [ ] Implement textDocument/didClose cleanup
- [ ] Manage document versions correctly
- [ ] Integrate with tree-sitter incremental parsing
- [ ] Maintain document cache with LRU eviction

**Technical Notes:**
- Use lsp-textdocument for version management
- Ensure thread-safe document access
- Optimize for rapid typing scenarios

## Story 2.3: Create Basic Syntax Error Diagnostics
**Description:** Report syntax errors from tree-sitter parsing as LSP diagnostics.

**Acceptance Criteria:**
- [ ] Extract error nodes from tree-sitter parse tree
- [ ] Convert parse errors to LSP Diagnostic format
- [ ] Publish diagnostics via textDocument/publishDiagnostics
- [ ] Clear diagnostics when errors are fixed
- [ ] Include helpful error messages
- [ ] Report error ranges accurately

**Technical Notes:**
- Batch diagnostic updates for performance
- Provide Gren-specific error messages
- Consider error recovery strategies

## Story 2.4: Implement Symbol Extraction
**Description:** Extract symbols (functions, types, modules) from parsed AST for indexing.

**Acceptance Criteria:**
- [ ] Create tree-sitter queries for Gren symbols
- [ ] Extract function definitions with signatures
- [ ] Identify type definitions and aliases
- [ ] Capture module declarations and imports
- [ ] Store symbols in SQLite database
- [ ] Update symbols on file changes

**Technical Notes:**
- Design efficient tree-sitter query patterns
- Handle nested symbols correctly
- Maintain symbol relationships

## Story 2.5: Enable Basic Code Completion
**Description:** Provide code completion for symbols in the current file and imported modules.

**Acceptance Criteria:**
- [ ] Handle textDocument/completion requests
- [ ] Complete local symbols from current file
- [ ] Include imported symbols from other modules
- [ ] Provide completion item details (type, docs)
- [ ] Support keyword completion
- [ ] Filter results based on context

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
- [ ] Handle textDocument/hover requests
- [ ] Show type signatures for functions
- [ ] Display documentation if available
- [ ] Format hover content as Markdown
- [ ] Include module information
- [ ] Respond quickly to hover requests

**Technical Notes:**
- Extract doc comments from source
- Cache hover information
- Keep hover content concise

## Story 2.8: Create Workspace Symbol Index
**Description:** Build and maintain a searchable index of all symbols in the workspace.

**Acceptance Criteria:**
- [ ] Index all Gren files on workspace initialization
- [ ] Update index on file changes
- [ ] Store symbols with metadata in SQLite
- [ ] Implement efficient symbol search queries
- [ ] Handle workspace folder changes
- [ ] Provide progress reporting for indexing

**Technical Notes:**
- Use parallel processing for initial indexing
- Implement incremental index updates
- Design for large workspace scalability
