# Gren LSP Test Cases

This document defines the core LSP messages that the Gren Language Server will support, along with JSON RPC test message sequences for each capability.

## Core LSP Messages for Gren

Based on the LSP specification and Gren's characteristics as a pure functional language with deterministic imports and no polymorphic overloading, the following messages are prioritized:

### 1. Server Lifecycle Messages

#### Initialize Request/Response
**Purpose**: Establish connection between client and server, negotiate capabilities.

**Test Sequence**:
```json
[
  {
    "sender": "client",
    "message": "initialize",
    "contents": {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "initialize",
      "params": {
        "processId": 12345,
        "clientInfo": {
          "name": "vscode",
          "version": "1.70.0"
        },
        "rootUri": "file:///workspace/gren-project",
        "capabilities": {
          "textDocument": {
            "synchronization": {
              "dynamicRegistration": false,
              "willSave": false,
              "willSaveWaitUntil": false,
              "didSave": false
            },
            "completion": {
              "dynamicRegistration": false,
              "completionItem": {
                "snippetSupport": true,
                "commitCharactersSupport": false,
                "documentationFormat": ["markdown", "plaintext"],
                "deprecatedSupport": false,
                "preselectSupport": false
              }
            },
            "hover": {
              "dynamicRegistration": false,
              "contentFormat": ["markdown", "plaintext"]
            },
            "definition": {
              "dynamicRegistration": false,
              "linkSupport": true
            },
            "references": {
              "dynamicRegistration": false
            },
            "documentSymbol": {
              "dynamicRegistration": false,
              "symbolKind": {
                "valueSet": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26]
              }
            }
          },
          "workspace": {
            "applyEdit": false,
            "workspaceEdit": {
              "documentChanges": false
            },
            "didChangeConfiguration": {
              "dynamicRegistration": false
            },
            "didChangeWatchedFiles": {
              "dynamicRegistration": false
            }
          }
        },
        "trace": "off"
      }
    }
  },
  {
    "sender": "server",
    "message": "initialize_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 1,
      "result": {
        "capabilities": {
          "textDocumentSync": {
            "openClose": true,
            "change": 2,
            "willSave": false,
            "willSaveWaitUntil": false,
            "save": {
              "includeText": false
            }
          },
          "completionProvider": {
            "resolveProvider": false,
            "triggerCharacters": ["."]
          },
          "hoverProvider": true,
          "definitionProvider": true,
          "referencesProvider": true,
          "documentSymbolProvider": true,
          "workspaceSymbolProvider": false,
          "codeActionProvider": false,
          "codeLensProvider": null,
          "documentFormattingProvider": false,
          "documentRangeFormattingProvider": false,
          "renameProvider": false,
          "documentLinkProvider": null,
          "executeCommandProvider": null,
          "experimental": null
        },
        "serverInfo": {
          "name": "gren-lsp",
          "version": "0.1.0"
        }
      }
    }
  }
]
```

#### Initialized Notification
**Purpose**: Sent by client to indicate it has received the initialize response.

**Test Sequence**:
```json
[
  {
    "sender": "client",
    "message": "initialized",
    "contents": {
      "jsonrpc": "2.0",
      "method": "initialized",
      "params": {}
    }
  }
]
```

#### Shutdown Request/Response
**Purpose**: Gracefully shut down the server.

**Test Sequence**:
```json
[
  {
    "sender": "client",
    "message": "shutdown",
    "contents": {
      "jsonrpc": "2.0",
      "id": 42,
      "method": "shutdown",
      "params": null
    }
  },
  {
    "sender": "server",
    "message": "shutdown_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 42,
      "result": null
    }
  }
]
```

#### Exit Notification
**Purpose**: Final message to terminate the server process.

**Test Sequence**:
```json
[
  {
    "sender": "client",
    "message": "exit",
    "contents": {
      "jsonrpc": "2.0",
      "method": "exit"
    }
  }
]
```

### 2. Text Document Synchronization

#### textDocument/didOpen
**Purpose**: Client informs server that a document has been opened.

**Test Sequence**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/didOpen",
    "contents": {
      "jsonrpc": "2.0",
      "method": "textDocument/didOpen",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren",
          "languageId": "gren",
          "version": 1,
          "text": "module Main exposing (main)\n\nimport Html exposing (text)\n\nmain =\n    text \"Hello, Gren!\"\n"
        }
      }
    }
  }
]
```

#### textDocument/didChange
**Purpose**: Client informs server of document content changes.

**Test Sequence - Incremental Changes**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/didChange",
    "contents": {
      "jsonrpc": "2.0",
      "method": "textDocument/didChange",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren",
          "version": 2
        },
        "contentChanges": [
          {
            "range": {
              "start": {
                "line": 4,
                "character": 7
              },
              "end": {
                "line": 4,
                "character": 7
              }
            },
            "text": " world"
          }
        ]
      }
    }
  }
]
```

**Test Sequence - Full Document Sync**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/didChange",
    "contents": {
      "jsonrpc": "2.0",
      "method": "textDocument/didChange",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren",
          "version": 3
        },
        "contentChanges": [
          {
            "text": "module Main exposing (main)\n\nimport Html exposing (text)\n\nmain =\n    text \"Hello, world!\"\n"
          }
        ]
      }
    }
  }
]
```

#### textDocument/didClose
**Purpose**: Client informs server that a document has been closed.

**Test Sequence**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/didClose",
    "contents": {
      "jsonrpc": "2.0",
      "method": "textDocument/didClose",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren"
        }
      }
    }
  }
]
```

### 3. Language Features

#### textDocument/completion
**Purpose**: Provide auto-completion suggestions at cursor position.

**State Requirements**: Requires one document to be opened with the test content.

**Setup Sequence**: Before each completion test, the following setup is required:
1. Initialize server
2. Send initialized notification  
3. Open document with test content

**Test Sequence - Module Member Completion**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/completion",
    "contents": {
      "jsonrpc": "2.0",
      "id": 10,
      "method": "textDocument/completion",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren"
        },
        "position": {
          "line": 2,
          "character": 17
        },
        "context": {
          "triggerKind": 2,
          "triggerCharacter": "."
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "completion_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 10,
      "result": {
        "isIncomplete": false,
        "items": [
          {
            "label": "text",
            "kind": 3,
            "detail": "String -> Html msg",
            "documentation": {
              "kind": "markdown",
              "value": "Create a text node with the given content."
            },
            "insertText": "text"
          },
          {
            "label": "div",
            "kind": 3,
            "detail": "Array (Attribute msg) -> Array (Html msg) -> Html msg",
            "documentation": {
              "kind": "markdown", 
              "value": "Create a div element."
            },
            "insertText": "div"
          }
        ]
      }
    }
  }
]
```

**Test Sequence - Variable Completion**:
*Note: Requires same setup sequence as Module Member Completion above*

```json
[
  {
    "sender": "client",
    "message": "textDocument/completion",
    "contents": {
      "jsonrpc": "2.0",
      "id": 11,
      "method": "textDocument/completion",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren"
        },
        "position": {
          "line": 5,
          "character": 8
        },
        "context": {
          "triggerKind": 1
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "completion_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 11,
      "result": {
        "isIncomplete": false,
        "items": [
          {
            "label": "text",
            "kind": 3,
            "detail": "String -> Html msg",
            "insertText": "text"
          }
        ]
      }
    }
  }
]
```

#### textDocument/hover
**Purpose**: Show type information and documentation when hovering over symbols.

**State Requirements**: Requires one document to be opened with the test content.

**Setup Sequence**: Before each hover test, the following setup is required:
1. Initialize server
2. Send initialized notification  
3. Open document with test content

**Test Sequence - Function Hover**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/hover",
    "contents": {
      "jsonrpc": "2.0",
      "id": 20,
      "method": "textDocument/hover",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren"
        },
        "position": {
          "line": 5,
          "character": 4
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "hover_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 20,
      "result": {
        "contents": {
          "kind": "markdown",
          "value": "```gren\ntext : String -> Html msg\n```\n\nCreate a text node with the given content.\n\n**Module:** Html"
        },
        "range": {
          "start": {
            "line": 5,
            "character": 4
          },
          "end": {
            "line": 5, 
            "character": 8
          }
        }
      }
    }
  }
]
```

**Test Sequence - Variable Hover**:
*Note: Requires same setup sequence as Function Hover above*

```json
[
  {
    "sender": "client",
    "message": "textDocument/hover",
    "contents": {
      "jsonrpc": "2.0",
      "id": 21,
      "method": "textDocument/hover",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren"
        },
        "position": {
          "line": 4,
          "character": 0
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "hover_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 21,
      "result": {
        "contents": {
          "kind": "markdown",
          "value": "```gren\nmain : Html Never\n```\n\nThe main function for this Gren program."
        },
        "range": {
          "start": {
            "line": 4,
            "character": 0
          },
          "end": {
            "line": 4,
            "character": 4
          }
        }
      }
    }
  }
]
```

#### textDocument/definition
**Purpose**: Navigate to the definition of a symbol.

**State Requirements**: Requires one document for local definitions, or multiple documents for cross-module definitions.

**Setup Sequence**: Before each definition test, the following setup is required:
1. Initialize server
2. Send initialized notification  
3. Open document(s) with test content

**Test Sequence - Go to Function Definition** (External Package):
*Note: This test references a function from the Html package, so no additional document setup needed*
```json
[
  {
    "sender": "client",
    "message": "textDocument/definition",
    "contents": {
      "jsonrpc": "2.0",
      "id": 30,
      "method": "textDocument/definition",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Main.gren"
        },
        "position": {
          "line": 5,
          "character": 4
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "definition_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 30,
      "result": [
        {
          "uri": "file:///workspace/gren-project/.gren/packages/gren-lang/html/1.0.0/src/Html.gren",
          "range": {
            "start": {
              "line": 42,
              "character": 0
            },
            "end": {
              "line": 42,
              "character": 4
            }
          }
        }
      ]
    }
  }
]
```

**Test Sequence - Go to Local Definition**:
*Note: This test requires Utils.gren to be opened with the appropriate test content containing the function definition at line 5*

```json
[
  {
    "sender": "client",
    "message": "textDocument/definition",
    "contents": {
      "jsonrpc": "2.0",
      "id": 31,
      "method": "textDocument/definition",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Utils.gren"
        },
        "position": {
          "line": 10,
          "character": 4
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "definition_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 31,
      "result": [
        {
          "uri": "file:///workspace/gren-project/src/Utils.gren",
          "range": {
            "start": {
              "line": 5,
              "character": 0
            },
            "end": {
              "line": 5,
              "character": 6
            }
          }
        }
      ]
    }
  }
]
```

#### textDocument/references
**Purpose**: Find all references to a symbol.

**State Requirements**: Requires multiple documents to be opened - Utils.gren (containing the definition), Main.gren and App.gren (containing references).

**Setup Sequence**: Before this references test, the following setup is required:
1. Initialize server
2. Send initialized notification  
3. Open Utils.gren with function definition
4. Open Main.gren with reference to the function
5. Open App.gren with reference to the function

**Test Sequence - Find Function References**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/references",
    "contents": {
      "jsonrpc": "2.0",
      "id": 40,
      "method": "textDocument/references",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Utils.gren"
        },
        "position": {
          "line": 5,
          "character": 3
        },
        "context": {
          "includeDeclaration": true
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "references_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 40,
      "result": [
        {
          "uri": "file:///workspace/gren-project/src/Utils.gren",
          "range": {
            "start": {
              "line": 5,
              "character": 0
            },
            "end": {
              "line": 5,
              "character": 6
            }
          }
        },
        {
          "uri": "file:///workspace/gren-project/src/Main.gren",
          "range": {
            "start": {
              "line": 8,
              "character": 4
            },
            "end": {
              "line": 8,
              "character": 10
            }
          }
        },
        {
          "uri": "file:///workspace/gren-project/src/App.gren",
          "range": {
            "start": {
              "line": 15,
              "character": 8
            },
            "end": {
              "line": 15,
              "character": 14
            }
          }
        }
      ]
    }
  }
]
```

#### textDocument/documentSymbol
**Purpose**: Provide outline/symbols for the current document.

**State Requirements**: Requires one document (Utils.gren) to be opened with test content containing functions and symbols.

**Setup Sequence**: Before this document symbols test, the following setup is required:
1. Initialize server
2. Send initialized notification  
3. Open Utils.gren with content containing helper and process functions

**Test Sequence - Document Symbols**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/documentSymbol",
    "contents": {
      "jsonrpc": "2.0",
      "id": 50,
      "method": "textDocument/documentSymbol",
      "params": {
        "textDocument": {
          "uri": "file:///workspace/gren-project/src/Utils.gren"
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "documentSymbol_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 50,
      "result": [
        {
          "name": "Utils",
          "kind": 9,
          "range": {
            "start": {
              "line": 0,
              "character": 0
            },
            "end": {
              "line": 20,
              "character": 0
            }
          },
          "selectionRange": {
            "start": {
              "line": 0,
              "character": 7
            },
            "end": {
              "line": 0,
              "character": 12
            }
          },
          "children": [
            {
              "name": "helper",
              "kind": 12,
              "range": {
                "start": {
                  "line": 5,
                  "character": 0
                },
                "end": {
                  "line": 7,
                  "character": 15
                }
              },
              "selectionRange": {
                "start": {
                  "line": 5,
                  "character": 0
                },
                "end": {
                  "line": 5,
                  "character": 6
                }
              }
            },
            {
              "name": "process",
              "kind": 12,
              "range": {
                "start": {
                  "line": 10,
                  "character": 0
                },
                "end": {
                  "line": 15,
                  "character": 20
                }
              },
              "selectionRange": {
                "start": {
                  "line": 10,
                  "character": 0
                },
                "end": {
                  "line": 10,
                  "character": 7
                }
              }
            }
          ]
        }
      ]
    }
  }
]
```

### 4. Diagnostics

#### textDocument/publishDiagnostics
**Purpose**: Server sends diagnostic information (errors, warnings) to client.

**State Requirements**: Requires a document to be opened and modified to contain errors that trigger diagnostics.

**Setup Sequence**: Before diagnostics tests, the following setup is required:
1. Initialize server
2. Send initialized notification  
3. Open document with valid content
4. Modify document to introduce errors (triggers diagnostic publication)

**Test Sequence - Syntax Error**:
```json
[
  {
    "sender": "server",
    "message": "textDocument/publishDiagnostics",
    "contents": {
      "jsonrpc": "2.0",
      "method": "textDocument/publishDiagnostics",
      "params": {
        "uri": "file:///workspace/gren-project/src/Main.gren",
        "version": 4,
        "diagnostics": [
          {
            "range": {
              "start": {
                "line": 5,
                "character": 4
              },
              "end": {
                "line": 5,
                "character": 8
              }
            },
            "severity": 1,
            "code": "NAMING_ERROR",
            "source": "gren",
            "message": "I cannot find a `texy` variable.\n\nDid you mean one of these?\n\n    text\n    Text\n\nHint: Read <https://gren-lang.org/book/naming> to learn about naming conventions in Gren. Names must be precise!"
          }
        ]
      }
    }
  }
]
```

**Test Sequence - Type Error**:
*Note: Requires same setup sequence as Syntax Error above, but with different content modification to trigger type errors*

```json
[
  {
    "sender": "server",
    "message": "textDocument/publishDiagnostics",
    "contents": {
      "jsonrpc": "2.0",
      "method": "textDocument/publishDiagnostics",
      "params": {
        "uri": "file:///workspace/gren-project/src/Main.gren",
        "version": 5,
        "diagnostics": [
          {
            "range": {
              "start": {
                "line": 5,
                "character": 9
              },
              "end": {
                "line": 5,
                "character": 11
              }
            },
            "severity": 1,
            "code": "TYPE_MISMATCH",
            "source": "gren",
            "message": "This `text` call produces:\n\n    Html msg\n\nBut the type annotation on `main` says it should be:\n\n    Int\n\nHint: Type annotations always override type inference. Make sure your annotation is correct!"
          }
        ]
      }
    }
  }
]
```

**Test Sequence - Clear Diagnostics**:
*Note: Requires document to be modified back to valid content to trigger diagnostic clearing*

```json
[
  {
    "sender": "server",
    "message": "textDocument/publishDiagnostics",
    "contents": {
      "jsonrpc": "2.0",
      "method": "textDocument/publishDiagnostics",
      "params": {
        "uri": "file:///workspace/gren-project/src/Main.gren",
        "version": 6,
        "diagnostics": []
      }
    }
  }
]
```

### 5. Error Handling

#### Invalid Request
**Test Sequence - Method Not Found**:
```json
[
  {
    "sender": "client",
    "message": "unsupported/method",
    "contents": {
      "jsonrpc": "2.0",
      "id": 99,
      "method": "unsupported/method",
      "params": {}
    }
  },
  {
    "sender": "server",
    "message": "error_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 99,
      "error": {
        "code": -32601,
        "message": "Method not found"
      }
    }
  }
]
```

#### Request Before Initialize
**Test Sequence - Request Before Initialize**:
```json
[
  {
    "sender": "client",
    "message": "textDocument/completion",
    "contents": {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "textDocument/completion",
      "params": {
        "textDocument": {
          "uri": "file:///test.gren"
        },
        "position": {
          "line": 0,
          "character": 0
        }
      }
    }
  },
  {
    "sender": "server",
    "message": "error_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 1,
      "error": {
        "code": -32002,
        "message": "Server not initialized"
      }
    }
  }
]
```

## Test Setup Helpers

To ensure test isolation and reduce boilerplate, the following setup helpers should be implemented:

### 1. Basic Server Setup
```json
// Helper: initializeServer()
[
  {
    "sender": "client",
    "message": "initialize",
    "contents": {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "initialize",
      "params": {
        "processId": 12345,
        "clientInfo": { "name": "test-client", "version": "1.0.0" },
        "rootUri": "file:///workspace/test-project",
        "capabilities": {
          "textDocument": {
            "synchronization": { "dynamicRegistration": false },
            "completion": { "dynamicRegistration": false },
            "hover": { "dynamicRegistration": false },
            "definition": { "dynamicRegistration": false },
            "references": { "dynamicRegistration": false },
            "documentSymbol": { "dynamicRegistration": false }
          }
        },
        "trace": "off"
      }
    }
  },
  {
    "sender": "server",
    "message": "initialize_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 1,
      "result": {
        "capabilities": {
          "textDocumentSync": { "openClose": true, "change": 2 },
          "completionProvider": { "triggerCharacters": ["."] },
          "hoverProvider": true,
          "definitionProvider": true,
          "referencesProvider": true,
          "documentSymbolProvider": true
        }
      }
    }
  },
  {
    "sender": "client",
    "message": "initialized",
    "contents": {
      "jsonrpc": "2.0",
      "method": "initialized",
      "params": {}
    }
  }
]
```

### 2. Document Setup Helpers

```json
// Helper: openMainGren()
{
  "sender": "client",
  "message": "textDocument/didOpen",
  "contents": {
    "jsonrpc": "2.0",
    "method": "textDocument/didOpen",
    "params": {
      "textDocument": {
        "uri": "file:///workspace/test-project/src/Main.gren",
        "languageId": "gren",
        "version": 1,
        "text": "module Main exposing (main)\n\nimport Html exposing (text)\n\nmain =\n    text \"Hello, Gren!\"\n"
      }
    }
  }
}
```

```json
// Helper: openUtilsGren()
{
  "sender": "client",
  "message": "textDocument/didOpen",
  "contents": {
    "jsonrpc": "2.0",
    "method": "textDocument/didOpen",
    "params": {
      "textDocument": {
        "uri": "file:///workspace/test-project/src/Utils.gren",
        "languageId": "gren",
        "version": 1,
        "text": "module Utils exposing (helper, process)\n\nimport String\n\n-- Helper function\nhelper : String -> String\nhelper input =\n    String.toUpper input\n\n-- Process function\nprocess : Array String -> Array String\nprocess items =\n    Array.map helper items\n"
      }
    }
  }
}
```

```json
// Helper: openAppGren()
{
  "sender": "client",
  "message": "textDocument/didOpen",
  "contents": {
    "jsonrpc": "2.0",
    "method": "textDocument/didOpen",
    "params": {
      "textDocument": {
        "uri": "file:///workspace/test-project/src/App.gren",
        "languageId": "gren",
        "version": 1,
        "text": "module App exposing (init)\n\nimport Utils exposing (helper)\n\ninit : String -> String\ninit input =\n    let\n        processed = helper input\n    in\n        processed\n"
      }
    }
  }
}
```

### 3. Content Modification Helpers

```json
// Helper: introduceTypoInMain() - Changes "text" to "texy" to trigger naming error
{
  "sender": "client",
  "message": "textDocument/didChange",
  "contents": {
    "jsonrpc": "2.0",
    "method": "textDocument/didChange",
    "params": {
      "textDocument": {
        "uri": "file:///workspace/test-project/src/Main.gren",
        "version": 2
      },
      "contentChanges": [
        {
          "range": {
            "start": { "line": 5, "character": 4 },
            "end": { "line": 5, "character": 8 }
          },
          "text": "texy"
        }
      ]
    }
  }
}
```

```json
// Helper: introduceTypeError() - Changes main signature to cause type mismatch
{
  "sender": "client",
  "message": "textDocument/didChange",
  "contents": {
    "jsonrpc": "2.0",
    "method": "textDocument/didChange",
    "params": {
      "textDocument": {
        "uri": "file:///workspace/test-project/src/Main.gren",
        "version": 2
      },
      "contentChanges": [
        {
          "range": {
            "start": { "line": 4, "character": 0 },
            "end": { "line": 4, "character": 6 }
          },
          "text": "main : Int\nmain"
        }
      ]
    }
  }
}
```

### 4. Cleanup Helper

```json
// Helper: cleanupServer()
[
  {
    "sender": "client",
    "message": "shutdown",
    "contents": {
      "jsonrpc": "2.0",
      "id": 999,
      "method": "shutdown",
      "params": null
    }
  },
  {
    "sender": "server",
    "message": "shutdown_response",
    "contents": {
      "jsonrpc": "2.0",
      "id": 999,
      "result": null
    }
  },
  {
    "sender": "client",
    "message": "exit",
    "contents": {
      "jsonrpc": "2.0",
      "method": "exit"
    }
  }
]
```

## Testing Strategy

Each test case should be implemented as follows:

1. **Unit Tests**: Test individual message handlers in isolation
2. **Integration Tests**: Test full message sequences using the stdio interface
3. **End-to-End Tests**: Test with actual Gren projects and real client interactions

### Test Isolation Requirements

**Critical**: Each test must be completely isolated and start from a clean server state. This means:

1. **No Shared State**: Each test spawns a fresh LSP server process
2. **Complete Setup**: Each test includes all necessary setup (initialize + document opening)
3. **Full Cleanup**: Each test properly shuts down the server process
4. **Deterministic Content**: All test documents use predefined, consistent content

### Test Framework Implementation

The test framework should:
- Send JSON-RPC messages via stdin to the LSP server process
- Capture stdout responses and parse JSON-RPC replies
- Assert that responses match expected message structure and content
- Support timeouts for requests that may not complete immediately
- Allow chaining multiple messages in sequence to test stateful interactions
- Provide helper functions for common setup sequences
- Ensure complete process isolation between tests

### Test File Organization

**Critical**: All test sequences should be written as static files on disk, not embedded as strings in code. This enables:
- Easy inspection of message sequences during development
- Version control tracking of test changes
- Manual debugging by examining exact JSON-RPC messages
- Collaborative review of test scenarios

#### Directory Structure
```
lsp/
├── test-cases.md                    # This documentation
├── tests/
│   ├── helpers/                     # Reusable setup sequences
│   │   ├── initialize-server.json   # Basic server initialization
│   │   ├── open-main-gren.json     # Open Main.gren with standard content
│   │   ├── open-utils-gren.json    # Open Utils.gren with test functions
│   │   ├── open-app-gren.json      # Open App.gren with references
│   │   ├── introduce-typo.json     # Modify document to create naming error
│   │   ├── introduce-type-error.json # Modify document to create type error
│   │   └── cleanup-server.json     # Shutdown and exit sequence
│   ├── lifecycle/                   # Server lifecycle tests
│   │   ├── initialize.json          # Initialize request/response test
│   │   ├── shutdown.json            # Shutdown request/response test
│   │   └── exit.json               # Exit notification test
│   ├── synchronization/             # Document sync tests
│   │   ├── did-open.json           # Document open test
│   │   ├── did-change-incremental.json # Incremental change test
│   │   ├── did-change-full.json    # Full document sync test
│   │   └── did-close.json          # Document close test
│   ├── completion/                  # Code completion tests
│   │   ├── module-member.json      # Completion after "Html."
│   │   └── variable.json           # Variable name completion
│   ├── hover/                       # Hover information tests
│   │   ├── function-hover.json     # Hover over function name
│   │   └── variable-hover.json     # Hover over variable
│   ├── definition/                  # Go-to-definition tests
│   │   ├── external-package.json   # Navigate to package function
│   │   └── local-definition.json   # Navigate to local function
│   ├── references/                  # Find references tests
│   │   └── function-references.json # Find all function references
│   ├── symbols/                     # Document symbols tests
│   │   └── document-symbols.json   # Get document outline
│   ├── diagnostics/                 # Error reporting tests
│   │   ├── syntax-error.json       # Naming/syntax error reporting
│   │   ├── type-error.json         # Type mismatch error reporting
│   │   └── clear-diagnostics.json  # Error clearing
│   └── errors/                      # Error handling tests
│       ├── method-not-found.json   # Unsupported method handling
│       └── before-initialize.json  # Request before initialization
```

#### Test File Format
Each test file contains a JSON array of message exchanges with metadata:

```json
{
  "name": "Module Member Completion",
  "description": "Test completion suggestions after typing 'Html.' in import context",
  "setup_required": ["initialize-server", "open-main-gren"],
  "cleanup_required": ["cleanup-server"],
  "messages": [
    {
      "sender": "client",
      "message": "textDocument/completion",
      "contents": {
        "jsonrpc": "2.0",
        "id": 10,
        "method": "textDocument/completion",
        "params": {
          "textDocument": {
            "uri": "file:///workspace/test-project/src/Main.gren"
          },
          "position": {
            "line": 2,
            "character": 17
          },
          "context": {
            "triggerKind": 2,
            "triggerCharacter": "."
          }
        }
      }
    },
    {
      "sender": "server",
      "message": "completion_response",
      "contents": {
        "jsonrpc": "2.0",
        "id": 10,
        "result": {
          "isIncomplete": false,
          "items": [
            {
              "label": "text",
              "kind": 3,
              "detail": "String -> Html msg",
              "insertText": "text"
            }
          ]
        }
      }
    }
  ]
}
```

#### Templating Support (Minimal)
For cases where dynamic values are needed, use simple template placeholders:

```json
{
  "textDocument": {
    "uri": "{{TEST_PROJECT_ROOT}}/src/Main.gren"
  },
  "position": {
    "line": 2,
    "character": 17  
  }
}
```

**Template Variables**:
- `{{TEST_PROJECT_ROOT}}`: Base URI for test project
- `{{TIMESTAMP}}`: Current timestamp for unique IDs
- `{{TEST_ID}}`: Unique test run identifier

**Principle**: Keep templating minimal. Static content is preferred over dynamic generation.

#### Test Execution Flow
The test runner should operate as follows:

1. **Load Test File**: Read and parse the test JSON file
2. **Execute Setup**: Run each helper file listed in `setup_required` in order
3. **Execute Test Messages**: Send each message in the `messages` array and validate responses
4. **Execute Cleanup**: Run each helper file listed in `cleanup_required` in order
5. **Report Results**: Compare actual responses with expected responses

#### Example Test Execution
For the module completion test:
```bash
# Test runner executes:
1. lsp/tests/helpers/initialize-server.json     # Server setup
2. lsp/tests/helpers/open-main-gren.json       # Document setup  
3. lsp/tests/completion/module-member.json     # Actual test
4. lsp/tests/helpers/cleanup-server.json       # Cleanup
```

#### Test File Dependencies
Some tests require multiple documents. The references test would specify:
```json
{
  "name": "Find Function References",
  "setup_required": [
    "initialize-server", 
    "open-utils-gren",    // Contains the function definition
    "open-main-gren",     // Contains a reference 
    "open-app-gren"       // Contains another reference
  ],
  "cleanup_required": ["cleanup-server"],
  "messages": [...]
}
```

#### Benefits of File-Based Tests
- **Inspectable**: Developers can examine exact message sequences
- **Debuggable**: Manual testing by replaying file contents to server
- **Maintainable**: Easy to update test scenarios without code changes
- **Reviewable**: Test changes visible in pull requests
- **Portable**: Test files can be used by different test runners or tools
- **Documentable**: Files serve as examples of proper LSP usage

## Message Priority for Implementation

1. **Phase 1 (MVP)**: Server lifecycle + basic synchronization
   - initialize/initialized/shutdown/exit
   - textDocument/didOpen/didChange/didClose
   - textDocument/publishDiagnostics

2. **Phase 2 (Core Features)**: Essential language features
   - textDocument/completion
   - textDocument/hover
   - textDocument/definition

3. **Phase 3 (Advanced Features)**: Enhanced navigation
   - textDocument/references
   - textDocument/documentSymbol

This phased approach allows for incremental development and testing of the LSP server while maintaining a working implementation at each stage.