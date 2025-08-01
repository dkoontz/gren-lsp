For the test `should handle completion requests` I want to be more specific on what we are expecting. The current implementation allows for two different structures but there should only be 1 (unless you have a reason why it can be either)
```
const greetCompletion = completions.items.find(item =>
  item.label === 'greet' || (typeof item.label === 'object' && item.label.label === 'greet')
);
```
Can you run the test so it fails and log the actual output so you can see the format that is being returned?







## Strange error output

When I run `npm test` I see the test produce some errors that seem to be unexpected. I believe it's during the test where we intentionally kill the server process, but it seems like we're not detecting the connection is down and that is causing downstream failures.

 Extension Lifecycle & Server Management Tests
    ✔ LSP server binary should exist
    ✔ LSP server binary should be executable
    ✔ LSP server should respond to --help
    ✔ extension should be installed and discoverable
    ✔ extension should activate on Gren file (47ms)
    ✔ extension configuration should have valid default values
Sending notification failed.
Sending request failed.
Error: Unexpected SIGPIPE
        at process.<anonymous> (file:///Users/david/dev/gren-lsp/editor-extensions/vscode/.vscode-test/vscode-darwin-arm64-1.102.3/Visual%20Studio%20Code.app/Contents/Resources/app/out/bootstrap-fork.js:3:11865)
        at process.emit (node:events:518:28)
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 28 failed
Sending notification failed.
Sending cancellation messages for id 29 failed
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 30 failed
Sending notification failed.
Sending cancellation messages for id 31 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 32 failed
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 33 failed
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 34 failed
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 36 failed
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 35 failed
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 38 failed
Sending notification failed.
Sending cancellation messages for id 37 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 39 failed
Sending notification failed.
Sending cancellation messages for id 40 failed
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 41 failed
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 42 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 43 failed
Sending notification failed.
Sending cancellation messages for id 44 failed
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 47 failed
Sending notification failed.
Sending cancellation messages for id 45 failed
Sending notification failed.
Sending cancellation messages for id 46 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 50 failed
Sending notification failed.
Sending cancellation messages for id 48 failed
Sending notification failed.
Sending cancellation messages for id 49 failed
An unknown error occurred. Please consult the log for more details.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 53 failed
Sending notification failed.
Sending cancellation messages for id 51 failed
Sending notification failed.
Sending cancellation messages for id 52 failed
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 56 failed
Sending notification failed.
Sending cancellation messages for id 54 failed
Sending notification failed.
Sending cancellation messages for id 55 failed
Sending request failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 59 failed
Sending notification failed.
Sending cancellation messages for id 57 failed
Sending notification failed.
Sending cancellation messages for id 58 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 60 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 61 failed
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 62 failed
Sending notification failed.
Sending cancellation messages for id 63 failed
Sending notification failed.
Sending cancellation messages for id 64 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 65 failed
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 66 failed
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 67 failed
Sending notification failed.
Sending cancellation messages for id 68 failed
Sending notification failed.
Sending cancellation messages for id 69 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 70 failed
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 71 failed
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 72 failed
Sending notification failed.
Sending cancellation messages for id 73 failed
Sending notification failed.
Sending cancellation messages for id 74 failed
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 77 failed
Sending notification failed.
Sending cancellation messages for id 75 failed
Sending notification failed.
Sending cancellation messages for id 76 failed
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending cancellation messages for id 78 failed
Sending notification failed.
Sending cancellation messages for id 79 failed
Sending notification failed.
Sending notification failed.
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 80 failed
Sending notification failed.
Sending cancellation messages for id 81 failed
Sending notification failed.
Sending request failed.
Sending request failed.
Sending notification failed.
Sending notification failed.
Sending cancellation messages for id 82 failed
Sending request failed.
rejected promise not handled within 1 second: Error [ERR_STREAM_DESTROYED]: Cannot call write after a stream was destroyed
stack trace: Error: Cannot call write after a stream was destroyed
        at _write (node:internal/streams/writable:491:11)
        at Writable.write (node:internal/streams/writable:510:10)
        at /Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/node/ril.js:88:29
        at new Promise (<anonymous>)
        at WritableStreamWrapper.write (/Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/node/ril.js:78:16)
        at StreamMessageWriter.doWrite (/Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/common/messageWriter.js:99:33)
        at /Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/common/messageWriter.js:90:29
rejected promise not handled within 1 second: Error [ERR_STREAM_DESTROYED]: Cannot call write after a stream was destroyed
stack trace: Error: Cannot call write after a stream was destroyed
        at _write (node:internal/streams/writable:491:11)
        at Writable.write (node:internal/streams/writable:510:10)
        at /Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/node/ril.js:88:29
        at new Promise (<anonymous>)
        at WritableStreamWrapper.write (/Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/node/ril.js:78:16)
        at StreamMessageWriter.doWrite (/Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/common/messageWriter.js:99:33)
        at /Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/common/messageWriter.js:90:29
rejected promise not handled within 1 second: Error [ERR_STREAM_DESTROYED]: Cannot call write after a stream was destroyed

...repeated 50x more times...

rejected promise not handled within 1 second: Error [ERR_STREAM_DESTROYED]: Cannot call write after a stream was destroyed
stack trace: Error: Cannot call write after a stream was destroyed
        at _write (node:internal/streams/writable:491:11)
        at Writable.write (node:internal/streams/writable:510:10)
        at /Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/node/ril.js:88:29
        at new Promise (<anonymous>)
        at WritableStreamWrapper.write (/Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/node/ril.js:78:16)
        at StreamMessageWriter.doWrite (/Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/common/messageWriter.js:99:33)
        at /Users/david/dev/gren-lsp/editor-extensions/vscode/node_modules/vscode-jsonrpc/lib/common/messageWriter.js:90:29
    ✔ should handle server process failure gracefully (13355ms)
