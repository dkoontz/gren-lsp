# VS Code Extension Testing

This document describes the automated test setup for the Gren LSP VS Code extension using vscode-extension-tester.

## Test Structure

The test suite is organized into several test files:

### 1. LSP Server Integration Tests (`src/test/suite/lsp-server.test.ts`)
- Verifies LSP server binary exists and is executable
- Tests basic server functionality (help command)
- **Must run first** - all other tests depend on the LSP server

### 2. File Opening Tests (`src/test/suite/file-opening.test.ts`)
- Tests opening `.gren` files
- Verifies language identification
- Confirms extension activation
- Tests with both simple test file and real project files

### 3. Text Editing Tests (`src/test/suite/text-editing.test.ts`)
- Tests basic text insertion, replacement, and deletion
- Verifies document synchronization with LSP server
- Tests undo/redo operations
- Tests rapid editing to ensure stability

### 4. Symbol Focusing Tests (`src/test/suite/symbol-focusing.test.ts`)
- Tests LSP features like hover information
- Tests go-to-definition functionality
- Tests find references
- Tests document symbols and workspace symbols
- Tests code completion
- Tests diagnostics
- **Note**: These tests are designed to gracefully handle missing LSP features

## Running Tests

### Prerequisites
1. **Close all VS Code instances** - Tests cannot run while VS Code is open
2. **Build the LSP server**: Run `just build` from the project root
3. **Install dependencies**: Run `npm install` in the extension directory

### Commands
```bash
# Run all tests
npm test

# Run UI tests specifically
npm test:ui

# Compile extension (run automatically before tests)
npm run compile
```

### Test Environment
- Tests automatically download and use a specific VS Code version (1.102.3)
- Tests run in a headless environment with the extension loaded
- Test workspace is created in `test-workspace/` with:
  - Simple Gren test file (`simple.gren`)
  - VS Code settings configured for verbose LSP tracing
  - Basic `gren.json` project configuration

## Test Data

### Simple Test File (`test-workspace/simple.gren`)
A basic Gren file with:
- Module declaration
- Function definitions (`sayHello`, `greet`)
- Node.js program structure
- Used for basic functionality testing

### Real Project Files
Tests also attempt to use files from `test-projects/application/` for more comprehensive testing.

## Configuration

### VS Code Settings (`test-workspace/.vscode/settings.json`)
- `grenLsp.trace.server`: "verbose" - Enable detailed LSP communication logging
- `grenLsp.debug.exportParseTree`: false - Disable parse tree export in tests
- `grenLsp.compiler.autoDownload`: true - Enable automatic compiler download

### Test Configuration
- `.vscode-test.mjs`: Main test configuration
- `.mocharc.json`: Mocha test runner configuration
- Timeout: 20 seconds per test (LSP startup can be slow)
- Test framework: Mocha with TDD interface

## Troubleshooting

### Common Issues

1. **"Running extension tests from the command line is currently only supported if no other instance of Code is running"**
   - Close all VS Code windows/instances
   - Quit VS Code completely
   - Re-run the tests

2. **"LSP server binary not found"**
   - Run `just build` from the project root
   - Verify the binary exists at `target/debug/gren-lsp`

3. **Tests timeout**
   - LSP server startup can be slow on first run
   - Increase timeout if needed in test files
   - Check LSP server logs for connection issues

4. **Extension not activating**
   - Verify `package.json` has correct activation events
   - Check that `.gren` files are properly recognized
   - Review extension logs in test output

### Debug Mode
To run tests with more verbose output:
1. Set `grenLsp.trace.server` to "verbose" in test workspace settings
2. Check console output during test runs
3. LSP communication will be logged in detail

## Extending Tests

### Adding New Test Files
1. Create new `.test.ts` files in `src/test/suite/`
2. Follow the existing pattern with `suite()` and `test()` functions
3. Import required VS Code APIs and test utilities
4. Tests will be automatically discovered and run

### Test Patterns
- Use `before()` for setup that runs once per suite
- Use `beforeEach()` for setup that runs before each test
- Use `after()` and `afterEach()` for cleanup
- Handle missing LSP features gracefully (log and continue, don't fail)
- Use appropriate timeouts for LSP operations

### LSP Feature Testing
When testing LSP features that may not be implemented:
- Wrap in try-catch blocks
- Log when features are missing rather than failing tests
- Use `this.skip()` to skip tests that can't run in the current environment
- Test both success and graceful failure cases

## Integration with Development Workflow

The test suite is designed to catch regressions and verify that:
1. The extension activates properly
2. Files are recognized and opened correctly
3. Basic text editing works without crashes
4. LSP server communication is stable
5. Advanced LSP features work when implemented

Run tests regularly during development, especially:
- After changes to extension activation logic
- After LSP server modifications
- Before creating releases
- When adding new language features