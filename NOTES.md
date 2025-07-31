Now we need to look at the way the tests are being carried out. Look at ./editor-extensions/vscode/src/test/suite/lsp-protocol.test.ts. The tests in this file are not actually asserting anything about the LSP although that is what they state they are doing. Let's go one by one:

- should send textDocument/didOpen when opening Gren file
What it does: this test opens a test file, gets the extension, but then simply checks if some text is in the file
What it should do: after getting the extension, the test should validate that it is was loaded correctly and is active (no errors starting u)

- should send textDocument/didChange when editing Gren file
What it does: this test inserts a block of text into an existing document, then checks if the file contains the new text
What it should do: after inserting the new contents, it should validate that the LSP was sent an event and then responded with the correct data (probably saying there were no diagnostics since the file doesn't have any errors). Also the delay time on LSP processing can be much lower, 1000ms should be plenty

- should handle hover requests on symbols
What it does: sets the cursor at a specific location and triggers the hover action in VS Code, then it doesn't assert anything
What it should do: after triggering the hover it should wait for a short delay (1000ms) and then assert that the LSP received the request and responded with the expected data for the symbol

- should handle go-to-definition requests
What it does: set the cursor at a symbol and triggers the go-to-definition action, then it doesn't assert anything
What is should do: after triggering the go-to-definition action, delay slightly, then assert the cursor has moved to the correct position in the document


## More tests that allow bad states
I am continuing to see test cases that excuse failure states. For example: `Extension is not active - may affect protocol tests`. If the extension is not active, and these are tests to validate the extension, then WHAT ARE WE EVEN TESTING? We absolutely must get rid of any cases like this. If the test is to validate the extension is loaded, then anything other than the extension being loaded is a failure.

## Change logging levels for VS Code extension

There is a lot of info currently being logged in the VS Code extension. This info is really valuable when debugging but overwhelming when just checking to see if something is happening. I want to move the log level of several events from info to debug.

This message is great at INFO, it informs the user that a top-level event (looking up a symbol) is happening.


`2025-07-30T18:48:30.097254Z  INFO gren_lsp_protocol::handlers: Searching for 'Tetromino' in module 'Dedris'`

The next few messages are more suited to debugging and therefore should be at the DEBUG log level.

```
2025-07-30T18:48:30.097531Z  INFO gren_lsp_protocol::handlers: 🔍 Checking if symbol 'Tetromino' in file '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' matches module path 'Dedris'
2025-07-30T18:48:30.097551Z  INFO gren_lsp_protocol::handlers: ❌ Pattern 1 failed: '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' does not end with '/Dedris.gren'
2025-07-30T18:48:30.097561Z  INFO gren_lsp_protocol::handlers: ❌ Pattern 2 failed: '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' does not end with '/Dedris.gren'
2025-07-30T18:48:30.097574Z  INFO gren_lsp_protocol::handlers: ❌ Symbol in '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' does not match required module path 'Dedris'
2025-07-30T18:48:30.097582Z  INFO gren_lsp_protocol::handlers: 🔍 Checking if symbol 'Tetromino' in file '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' matches module path 'Dedris'
2025-07-30T18:48:30.097588Z  INFO gren_lsp_protocol::handlers: ❌ Pattern 1 failed: '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' does not end with '/Dedris.gren'
2025-07-30T18:48:30.097595Z  INFO gren_lsp_protocol::handlers: ❌ Pattern 2 failed: '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' does not end with '/Dedris.gren'
2025-07-30T18:48:30.097601Z  INFO gren_lsp_protocol::handlers: ❌ Symbol in '/Users/david/dev/gren-lsp-test-projects/tetris/src/Dedris/Tetromino.gren' does not match required module path 'Dedris'
2025-07-30T18:48:30.097610Z  INFO gren_lsp_protocol::handlers: No qualified matches found for 'Tetromino' in module 'Dedris' - returning empty to avoid incorrect results
2025-07-30T18:48:30.097618Z  INFO gren_lsp_protocol::handlers: No hover content generated for 'Tetromino'
```

## VS Code extension doesn't set environment variable
The GREN_COMPILER_PATH is not being set by the VS Code extension causing the LSP server to fail. The extension should only start the server after downloading a Gren compiler and then must set the environment variable.
