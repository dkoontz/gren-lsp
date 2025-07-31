## Enhance VS Code extension to show actionable compiler errors as popup notifications
For some errors it would be really useful to show the user a popup instead of just a red squiggle line. This is a compiler error that is not currently detected. I want to make sure:

- This error is detected and not ignored
- There is a notification shown to the user informing them that there is compiler version mismatch with their project. It should direct them to either change compiler versions (give npm command to do this) or to change their `gren.json` compiler version.
- 
```
2025-07-30T18:40:12.047169Z  INFO gren_lsp_core::compiler: Compiler stderr: {"type":"error","path":"gren.json","title":"GREN VERSION MISMATCH","message":["Your gren.json says this application needs a different version of Gren.\n\nIt requires ",{"bold":false,"underline":false,"color":"GREEN","string":"0.4.4"},", but you are using ",{"bold":false,"underline":false,"color":"RED","string":"0.5.3"}," right now."]}
2025-07-30T18:40:12.047204Z  WARN gren_lsp_core::compiler: Unknown compiler output format, ignoring
2025-07-30T18:40:12.047209Z  INFO gren_lsp_core::compiler: 📋 Found 0 compiler diagnostics
```

For testing, the Tetris app had `"gren-version" : "0.4.4"`

## Compilation finds different version of Gren compiler
If I run `gren --version` from my terminal I get `0.5.4`. In the LSP server logs I am seeing this error message:

```
2025-07-30T18:54:48.183097Z  INFO gren_lsp_core::compiler: Compiler stderr: {"type":"error","path":"gren.json","title":"GREN VERSION MISMATCH","message":["Your gren.json says this application needs a different version of Gren.\n\nIt requires ",{"bold":false,"underline":false,"color":"GREEN","string":"0.5.4"},", but you are using ",{"bold":false,"underline":false,"color":"RED","string":"0.5.3"}," right now."]}
```

Is the LSP server somehow using a different version of the Gren compiler than the one I have on my path?


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

## Module import has broken some type lookup (might be fixed by compiler version issue)
After implementing story 3.5 some of the type lookups are failing. Given this code, when I hover over the `Tetromino` on the line `initL : Tetromino` I see this in the logs.

```gren
init : {} -> { model : Model , command : Cmd Msg }
init {} =
    { model = Model.init
    , command =
        Cmd.batch
            -- todo: Warum geht das nicht richtig mit nur einem Aufruf?
            [ Cmd.generateNewTetromino
            , Cmd.generateNewTetromino
            , Cmd.getViewport
            ]
    }

initL : Tetromino
initL =
    Dedris.Tetromino.l
```
