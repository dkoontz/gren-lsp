# Data Flow Architecture

## 1. Request Processing Flow

```
Client Request → LSP Service → Message Router → Feature Handler
                    ↓
              Document Manager ← Tree-sitter Parser
                    ↓
                Symbol Index → Query Results
                    ↓
              Compiler Interface (if needed)
                    ↓
              Response Formation → LSP Service → Client
```

## 2. Document Synchronization Flow

```
didOpen/didChange → lsp-textdocument → Document Manager → Tree-sitter Update
                         ↓                    ↓
                   UTF-16 Position       Position/Offset
                    Handling              Conversion
                         ↓                    ↓
                   Symbol Index Update ← Parse Tree
                         ↓
                  Compiler Invocation (async)
                         ↓
               Diagnostics Publication → Client
```

## 3. Symbol Resolution Flow

```
Cursor Position → Tree-sitter Query → Local Symbol?
                        ↓                    ↓
                   Symbol Index Query    Return Local
                        ↓
                Cross-Module Resolution
                        ↓
                Package Resolution
                        ↓
                  Final Location
```
