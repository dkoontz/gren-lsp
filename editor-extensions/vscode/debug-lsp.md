# LSP Document Change Debugging

## Issue
Document changes are not triggering compilation in the development environment, but they work in tests.

## Evidence from logs
1. ✅ Server advertises `TextDocumentSyncKind::INCREMENTAL` support
2. ✅ Server has proper `did_change` handler that logs "Document changed"
3. ❌ No "Document changed" log entries after making edits
4. ✅ Verbose tracing is enabled but LSP protocol messages aren't visible

## Quick Debug Steps

### 1. Verify LSP Protocol Messages
With verbose tracing enabled, you should see JSON-RPC messages like:
```
[Trace - 6:58:44 PM] Sending request 'textDocument/didChange - (1)'.
```

If you don't see these messages, the LSP client isn't sending document changes.

### 2. Check File Association
Ensure the file is properly recognized as a Gren file:
- Check that the status bar shows "Gren" as the language
- Try manually setting the language: Ctrl/Cmd+Shift+P → "Change Language Mode" → "Gren"

### 3. Test Document Sync Manually
1. Open VS Code Developer Tools (Help → Toggle Developer Tools)
2. In Console, run:
   ```javascript
   // Check if extension is active
   vscode.extensions.getExtension('gren-lsp.gren-lsp')?.isActive
   
   // Check active text editor language
   vscode.window.activeTextEditor?.document.languageId
   ```

### 4. Compare with Test Environment
The key difference might be:
- Test environment: Uses clean VS Code instance with only Gren extension loaded
- Development environment: Has other extensions that might interfere

### 5. Try Clean Environment
1. Disable all extensions except Gren LSP
2. Restart VS Code
3. Test document changes again

## Expected Behavior
When typing in a Gren file, you should see:
1. LSP protocol messages in the output (with verbose tracing)
2. "Document changed" messages in server logs
3. Compilation triggered after changes