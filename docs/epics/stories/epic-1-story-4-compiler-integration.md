# Epic 1 Story 4: Basic Compiler Integration & Diagnostics

## 📋 User Story
**As a** Gren developer  
**I want** to see syntax and type errors from the Gren compiler in my editor  
**So that** I can fix issues without switching to terminal

## ✅ Acceptance Criteria
- [ ] Compiler interface implemented for external Gren compiler process
- [ ] GREN_COMPILER_PATH environment variable support
- [ ] Temporary file creation for compilation
- [ ] Compiler output parsing into LSP diagnostics
- [ ] publishDiagnostics notifications sent to client
- [ ] Error recovery when compiler not found/fails

## 🧪 Integration Test Requirements

### Test: Compiler Process Management
- [ ] Test compiler process spawning and lifecycle
- [ ] Test compiler path resolution from environment variable
- [ ] Test handling of missing compiler binary
- [ ] Test compiler process timeout and cleanup

### Test: Temporary File Handling
- [ ] Test temporary file creation for in-memory documents
- [ ] Test file cleanup after compilation
- [ ] Test handling of file system errors
- [ ] Test concurrent compilation isolation

### Test: Compiler Output Parsing
- [ ] Test parsing syntax error messages into diagnostics
- [ ] Test parsing type error messages into diagnostics
- [ ] Test diagnostic position mapping accuracy
- [ ] Test handling of malformed compiler output

### Test: Diagnostics Publication
- [ ] Test publishDiagnostics messages sent to client
- [ ] Test diagnostic clearing when errors resolved
- [ ] Test diagnostic updates on document changes
- [ ] Test diagnostic message format and content

### Test: Error Recovery
- [ ] Test graceful handling of compiler crashes
- [ ] Test recovery from compiler unavailability
- [ ] Test handling of compilation timeouts
- [ ] Test server stability during compiler errors

## ✅ Definition of Done
- Syntax errors appear in editor as diagnostics
- Type errors reported with accurate positions  
- Diagnostics cleared when issues resolved
- Handles compiler failures gracefully
- No server crashes due to compiler issues
- All integration tests pass consistently

## 📁 Related Files
- `src/compiler_interface.rs` (TO BE CREATED)
- `src/diagnostics.rs` (TO BE CREATED)
- `tests/integration/compiler_integration_tests.rs` (TO BE CREATED)

## 🔗 Dependencies
- Epic 1 Story 3 completed (document lifecycle management)
- Gren compiler binary available
- Temporary file system access
- Process spawning capabilities

## 📊 Status
**Pending** - Awaiting Story 3 completion