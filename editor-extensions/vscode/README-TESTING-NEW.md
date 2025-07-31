# VS Code Extension Testing - Enhanced LSP Testing

This document describes the comprehensive automated test setup for the Gren LSP VS Code extension.

## Test Philosophy

Our tests focus on **actual LSP functionality** rather than basic VS Code features. They verify:
- LSP protocol message flow (`textDocument/didOpen`, `textDocument/didChange`, etc.)
- Diagnostic integration (compiler errors displayed in VS Code)
- Extension and server lifecycle management
- Real Gren language server functionality

## Test Structure

### **Core LSP Tests (NEW)**

#### 1. LSP Protocol Tests (`src/test/suite/lsp-protocol.test.ts`)
- **Purpose**: Verify LSP JSON-RPC message flow
- Tests:
  - `textDocument/didOpen` sent when opening Gren files
  - `textDocument/didChange` sent during edits
  - `textDocument/hover` requests and responses
  - `textDocument/definition` (go-to-definition)
  - `textDocument/completion` requests
  - Extension activation on Gren files

#### 2. Diagnostic Integration Tests (`src/test/suite/diagnostics.test.ts`)
- **Purpose**: Verify compiler errors appear as VS Code diagnostics
- Tests:
  - Syntax errors display correctly
  - Type errors show appropriate messages
  - Import errors are caught
  - Diagnostics clear when errors are fixed
  - No diagnostics for valid code
  - Diagnostic properties (source, severity, range)

#### 3. Server Integration Tests (`src/test/suite/server-integration.test.ts`)
- **Purpose**: Verify LSP server lifecycle and functionality
- Tests:
  - Server binary exists and is executable
  - Extension installation and activation
  - Configuration validation
  - Multiple file operations
  - Rapid file changes
  - Compiler integration

#### 4. LSP Server Binary Tests (`src/test/suite/lsp-server.test.ts`)
- **Purpose**: Basic server functionality verification
- Tests:
  - Server binary exists
  - Server is executable
  - Server responds to --help

### **Legacy Tests (Basic VS Code functionality)**

#### 5. Legacy File Opening Tests (`src/test/suite/legacy-file-opening.test.ts`)
- **Purpose**: Basic VS Code file operations (not LSP-specific)
- Tests opening `.gren` files and language identification

#### 6. Legacy Text Editing Tests (`src/test/suite/legacy-text-editing.test.ts`)
- **Purpose**: Basic VS Code text editing (not LSP-specific)
- Tests document editing, undo/redo operations

#### 7. Legacy Symbol Tests (`src/test/suite/legacy-symbol-focusing.test.ts`)
- **Purpose**: High-level VS Code API testing (not LSP protocol)
- Tests hover, definition, and completion via VS Code APIs

## Test Helpers

### LSP Message Monitor (`helpers/lsp-message-helper.ts`)
- Intercepts and monitors LSP JSON-RPC messages
- Waits for specific LSP methods to be called
- Tracks diagnostic changes
- Creates and cleans up test files

### Diagnostic Helper (`helpers/diagnostic-helper.ts`)
- Asserts diagnostic expectations
- Waits for diagnostics to match patterns
- Provides Gren-specific diagnostic matchers
- Logs diagnostics for debugging

### Test Files (`helpers/test-files/`)
- `syntax-error.gren`: Known syntax errors for testing
- `type-error.gren`: Type errors for diagnostic testing
- `import-error.gren`: Import errors for testing
- `valid-code.gren`: Valid Gren code (should produce no diagnostics)

## Key Testing Features

### 1. **Real LSP Protocol Verification**
Unlike the legacy tests that only verify VS Code's text editing works, the new tests verify:
- Actual LSP JSON-RPC messages are sent (`textDocument/didChange`)
- LSP server processes these messages
- Responses are handled correctly
- Diagnostics are published via `textDocument/publishDiagnostics`

### 2. **Diagnostic Integration Testing**
Tests create files with **known Gren errors** and verify:
- Compiler errors appear as VS Code diagnostics
- Error squiggles show up in the editor
- Problems panel displays the errors
- Diagnostics have correct source, severity, and location
- Errors clear when code is fixed

### 3. **Server Lifecycle Testing**
Tests verify:
- LSP server binary exists and runs
- Extension activates on Gren files
- Server handles multiple file operations
- Rapid changes don't crash the server
- Configuration is applied correctly

### 4. **Graceful Degradation**
Tests are designed to:
- Continue if LSP features aren't implemented yet
- Log what's working vs. what's not
- Provide debugging information
- Not fail on missing features during development

## Running Tests

### Prerequisites
1. **Close all VS Code instances** - Tests cannot run while VS Code is open
2. **Build the LSP server**: Run `just build` from the project root
3. **Install dependencies**: Run `npm install` in the extension directory

### Commands
```bash
# Build server and run all tests
just vscode-test

# Run tests manually (after building)
npm test

# Run specific test suite
npm test -- --grep "LSP Protocol"
npm test -- --grep "Diagnostic"
npm test -- --grep "Server Integration"
```

### Test Environment
- Tests automatically download VS Code test instance
- Tests run with only Gren LSP extension loaded (clean environment)
- Test workspace contains sample Gren files and configurations

## Expected Test Behavior

### ✅ **When LSP Integration is Working**
- **Protocol tests**: Show LSP messages being sent/received
- **Diagnostic tests**: Display compiler errors as VS Code diagnostics
- **Server tests**: Confirm server processes files and maintains state

### ⚠️ **During Development (LSP not fully implemented)**
- **Protocol tests**: Log "LSP integration pending" messages
- **Diagnostic tests**: Show "timeout waiting for diagnostics"
- **Server tests**: Verify server exists but may not process all requests

### ❌ **When Something is Broken**
- **Protocol tests**: Extension not loading or activating
- **Diagnostic tests**: No response from server at all
- **Server tests**: Binary missing or not executable

## Debugging Test Issues

### 1. **Extension Not Loading**
```bash
# Check extension installation
code --list-extensions | grep gren

# Clean and reinstall
just clean-dev-env
just vscode-dev-fresh
```

### 2. **No LSP Messages**
- Check "Gren LSP Extension" output channel
- Verify verbose tracing is enabled
- Look for server startup errors

### 3. **No Diagnostics**
- Check "Gren LSP Server" output channel
- Verify server is processing `textDocument/didChange`
- Check server logs in temp directory

### 4. **Server Binary Issues**
```bash
# Verify server exists and runs
just build
./target/debug/gren-lsp --help
```

## Test Development Guidelines

### When Adding New Tests
1. **Use test helpers** - Don't duplicate diagnostic/message monitoring logic
2. **Test real LSP behavior** - Verify protocol messages, not just VS Code APIs
3. **Handle missing features gracefully** - Log what's not implemented
4. **Clean up resources** - Always dispose monitors and delete test files
5. **Use meaningful assertions** - Provide detailed error messages

### Test File Organization
```
src/test/suite/
├── lsp-protocol.test.ts        # Core LSP message flow
├── diagnostics.test.ts         # Compiler error integration
├── server-integration.test.ts  # Server lifecycle & features
├── lsp-server.test.ts         # Basic server binary tests
├── legacy-*.test.ts           # Old basic VS Code tests
└── helpers/
    ├── lsp-message-helper.ts   # LSP protocol monitoring
    ├── diagnostic-helper.ts    # Diagnostic assertions
    └── test-files/             # Gren files with known errors
```

This test structure ensures we're testing the **actual LSP integration** rather than just VS Code's basic functionality, giving us confidence that the Gren language server is working correctly.