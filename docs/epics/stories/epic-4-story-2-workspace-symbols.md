# Epic 4 Story 2: Workspace Symbol Search

## 📋 User Story
**As a** Gren developer  
**I want** to search for symbols across my entire project workspace  
**So that** I can quickly navigate to any function, type, or module without knowing the exact file location

## ✅ Acceptance Criteria
- [ ] Implement workspace/symbol LSP handler with fuzzy search capability
- [ ] Index all symbols across workspace (functions, types, modules, constants)
- [ ] Support partial name matching and fuzzy search algorithms
- [ ] Return results with file location and symbol context
- [ ] Provide symbol kind classification (Function, Class, Variable, etc.)
- [ ] Support case-insensitive search with intelligent ranking
- [ ] Limit results to prevent overwhelming the user (max 50-100 results)
- [ ] Include symbol signature/type information in results

## 🧪 Integration Test Requirements

### Test: Basic Symbol Search
- [ ] Create workspace with multiple Gren files containing various symbols
- [ ] Search for exact function name and verify it's found
- [ ] Test search for type name returns correct definition location
- [ ] Validate module search returns module file location

### Test: Fuzzy Search Capabilities
- [ ] Search for partial symbol name (e.g., "userCr" finds "userCreate")
- [ ] Test case-insensitive matching ("USER" finds "userCreate")
- [ ] Verify fuzzy matching with typos ("usrCreate" finds "userCreate")
- [ ] Test that most relevant results appear first

### Test: Symbol Kind Classification
- [ ] Verify functions classified as Function kind
- [ ] Test types classified as Class kind (LSP convention)
- [ ] Test type constructors classified as Constructor kind
- [ ] Test constants classified as Variable kind
- [ ] Test modules classified as Module kind

### Test: Cross-File Symbol Search
- [ ] Create symbols across multiple files and directories
- [ ] Search for symbols defined in different modules
- [ ] Verify search includes symbols from subdirectories
- [ ] Test that all matching symbols are returned regardless of file location

### Test: Result Ranking and Filtering
- [ ] Search for common term that matches multiple symbols
- [ ] Verify exact matches ranked higher than partial matches
- [ ] Test that recently accessed symbols get priority boost
- [ ] Validate result limit prevents overwhelming response

### Test: Large Workspace Performance
- [ ] Create workspace with 50+ files and 500+ symbols
- [ ] Test search response time meets performance requirements
- [ ] Verify memory usage remains bounded during search
- [ ] Test that indexing doesn't block other LSP operations

### Test: LSP Protocol Compliance
- [ ] Validate JSON-RPC response format matches LSP 3.18 spec
- [ ] Test SymbolInformation structure with proper location data
- [ ] Verify containerName for nested symbols
- [ ] Test proper UTF-16 position encoding for symbol locations

## 🔧 Technical Implementation

### Symbol Indexing Strategy
- Extend existing SQLite symbol index for workspace-wide search
- Index symbol name, kind, location, and container information
- Support incremental updates when files change
- Implement efficient search queries with proper indexes

### Fuzzy Search Algorithm
- Implement fuzzy string matching with configurable tolerance
- Use relevance scoring based on match quality and symbol type
- Support case-insensitive search with intelligent ranking
- Cache search results for recently used queries

### LSP Handler Implementation
- Implement `workspace/symbol` message handler
- Convert internal symbol representation to LSP SymbolInformation
- Support query filtering and result limiting
- Handle empty queries (return most relevant/recent symbols)

### Workspace Integration
- Monitor workspace file changes for index updates
- Support multi-folder workspaces with proper scoping
- Respect .grenignore or similar exclusion patterns
- Handle workspace configuration changes

## ⚡ Performance Requirements
- Response time: < 300ms for 95% of queries in 100+ file projects
- Memory usage: Efficient index storage and search algorithms
- Support workspaces with 1000+ symbols effectively
- Incremental indexing to minimize startup impact

## ✅ Definition of Done
- workspace/symbol handler implemented and tested
- Fuzzy search provides relevant results for partial matches
- Symbol index covers all meaningful symbols across workspace
- Search results include accurate location and context information
- Integration tests validate search accuracy and performance with specific assertions
- Performance requirements met for large workspace scenarios
- Proper handling of workspace changes and symbol updates

## 📁 Related Files
- `src/workspace_symbols.rs` - Main WorkspaceSymbolEngine implementation
- `src/lsp_service.rs` - LSP handler integration and capability advertisement
- `src/workspace_symbols_integration_tests.rs` - Comprehensive test coverage
- Extensions to existing `src/symbol_index.rs` for workspace-wide indexing
- Integration with `src/workspace_management.rs` for file monitoring

## 🔗 Dependencies
- Epic 1-2 completed (LSP foundation, symbol indexing infrastructure)
- Existing SQLite symbol index for extension
- Workspace management system for file monitoring
- Symbol extraction from tree-sitter queries

## 📊 Status
**⏳ PENDING** - Ready for implementation

## 🎯 Success Metrics
- **Search Relevance**: 90% of searches return desired symbol in top 5 results
- **Performance**: Sub-300ms response time for typical workspace searches
- **Coverage**: Index includes 100% of meaningful symbols across workspace
- **Usability**: Fuzzy matching handles common typos and partial names

## 🔍 Search Examples

### Exact Name Search
```
Query: "createUser"
Results:
📍 createUser (Function) - src/User.gren:15
📍 createUserForm (Function) - src/Forms.gren:42
```

### Fuzzy Search
```
Query: "usrCr"
Results:
📍 createUser (Function) - src/User.gren:15
📍 userCreate (Function) - src/Actions.gren:28
📍 UserCreated (Constructor) - src/Events.gren:12
```

### Type Search
```
Query: "User"
Results:
📍 User (Type) - src/Types.gren:8
📍 UserRole (Type) - src/Auth.gren:15
📍 createUser (Function) - src/User.gren:15
📍 UserCreated (Constructor) - src/Events.gren:12
```

### Module Search
```
Query: "Http"
Results:
📍 Http (Module) - src/Http.gren:1
📍 HttpClient (Module) - src/Http/Client.gren:1
📍 makeHttpRequest (Function) - src/Api.gren:25
```

This story addresses the navigation challenge in large Gren projects by providing instant access to any symbol across the entire workspace, significantly improving developer productivity when working with codebases containing dozens or hundreds of files.