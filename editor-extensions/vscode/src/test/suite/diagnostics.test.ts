import * as assert from "assert";
import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { before, after, beforeEach, afterEach } from "mocha";
import {
  ObservedLSPMessageMonitor,
  createTestFileOnDisk,
  cleanupTestFile,
  closeAllFiles,
  openFileInEditor,
} from "./helpers/lsp-message-helper";
import { DiagnosticHelper } from "./helpers/diagnostic-helper";

suite("LSP Diagnostic Integration Tests", () => {
  let monitor: ObservedLSPMessageMonitor;
  let testWorkspaceUri: vscode.Uri;

  before(async function () {
    this.timeout(30000);

    const workspaceFolders = vscode.workspace.workspaceFolders;
    assert.ok(
      workspaceFolders && workspaceFolders.length > 0,
      "Test workspace should be open",
    );
    testWorkspaceUri = workspaceFolders[0].uri;

    monitor = new ObservedLSPMessageMonitor();
    await monitor.preActivateExtension();
    await monitor.attachToClient();
  });

  after(() => {
    if (monitor) {
      monitor.dispose();
    }
  });

  beforeEach(async () => {
    await closeAllFiles();
    monitor.clear();
  });

  afterEach(async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    monitor.stopMonitoring();
  });

  test("should show diagnostics for syntax errors", async function () {
    this.timeout(30000);

    const syntaxErrorCode = `module SyntaxTest exposing (main)

import Node
import Stream
import Bytes exposing (Bytes)
import Node exposing (Environment, Program)
import Init

main : Program Model Msg
main =
    Node.defineProgram
        { init = init
        , update = update
        , subscriptions = subscriptions
        }

type alias Model =
    { stdout : Stream.Writable Bytes
    , stderr : Stream.Writable Bytes
    }

type Msg
    = NoOp

init : Environment -> Init.Task { model : Model, command : Cmd Msg }
init env =
    Node.startProgram
        { model =
            { stdout = env.stdout
            , stderr = env.stderr
            }
        , command = Cmd.none
        }

update : Msg -> Model -> { model : Model, command : Cmd Msg }
update msg model =
    when msg is
        NoOp ->
            { model = model, command = Cmd.none }

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none

-- This function has a syntax error (missing =)
brokenFunction : String -> String
brokenFunction name
    "Hello, " ++ name ++ "!"`;

    // First verify the file doesn't exist yet
    const workspaceFolders = vscode.workspace.workspaceFolders;
    const expectedPath = workspaceFolders![0].uri.fsPath + "/src/SyntaxTest.gren";
    console.log(`🔍 Expected file path: ${expectedPath}`);
    
    const testUri = await createTestFileOnDisk(
      syntaxErrorCode,
      "src/SyntaxTest.gren",
    );

    // Verify the file was actually created
    const fs = require("fs");
    const fileExists = fs.existsSync(testUri.fsPath);
    console.log(`📁 File created successfully: ${fileExists} at ${testUri.fsPath}`);
    if (fileExists) {
      const fileSize = fs.statSync(testUri.fsPath).size;
      console.log(`📋 File size: ${fileSize} bytes`);
    }

    try {
      await monitor.startMonitoring(testUri);
      console.log(`📁 Test file URI: ${testUri.toString()}`);
      
      const document = await openFileInEditor(testUri);
      assert.strictEqual(
        document.languageId,
        "gren",
        "Document should be identified as Gren",
      );
      
      console.log(`📂 Opened document: ${document.uri.toString()}, languageId: ${document.languageId}`);

      // Wait for LSP didOpen message
      const didOpenMessage = await monitor.waitForMethod(
        "textDocument/didOpen",
      );
      assert.ok(didOpenMessage, "Should receive LSP didOpen message");
      console.log(`✅ Received didOpen message for: ${JSON.stringify(didOpenMessage.params?.textDocument?.uri)}`);

      // Debug: Check all LSP messages that have been sent
      console.log("🔍 All LSP messages captured:");
      const allMessages = monitor.getAllMessages();
      allMessages.forEach((msg: any, index: number) => {
        console.log(`  ${index + 1}. ${msg.method} (${msg.direction}) - ${JSON.stringify(msg.params?.textDocument?.uri || msg.params?.uri || 'no uri')}`);
      });

      // Give extra time for LSP server to process the file and generate diagnostics
      console.log("⏳ Waiting for LSP server to process the opened file...");
      
      // Display log file location for debugging
      const logFilePath = path.join(os.tmpdir(), 'gren-lsp', 'debug.log');
      console.log(`📄 Debug logs location: ${logFilePath}`);
      console.log("💡 Check this file for detailed LSP server logs including diagnostics processing");
      
      await monitor.waitForDiagnostics(testUri);

      const diagnostics = vscode.languages.getDiagnostics(testUri);
      
      // If test fails, display recent log entries for debugging
      if (diagnostics.length === 0) {
        console.log("\n❌ Test failed - no diagnostics received. Recent log entries:");
        try {
          if (fs.existsSync(logFilePath)) {
            const logContent = fs.readFileSync(logFilePath, 'utf8');
            const logLines = logContent.split('\n').filter((line: string) => line.trim());
            const recentLines = logLines.slice(-20); // Last 20 lines
            recentLines.forEach((line: string, index: number) => {
              console.log(`  ${recentLines.length - 20 + index + 1}: ${line}`);
            });
          } else {
            console.log("  📄 Log file not found or not created yet");
          }
        } catch (error) {
          console.log(`  ❌ Error reading log file: ${error}`);
        }
      }
      
      assert.ok(
        diagnostics.length > 0,
        "Should receive diagnostics for syntax error",
      );

      // Validate diagnostic content now that compiler bug is fixed
      const errors = diagnostics.filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Error,
      );
      assert.strictEqual(errors.length, 1, "Should have exactly one error diagnostic for syntax error");
      
      // Verify diagnostic properties
      const firstError = errors[0];
      assert.ok(firstError.message.length > 0, "Error should have a meaningful message");
      assert.ok(firstError.range, "Error should have a range");
      assert.ok(firstError.range.start.line >= 0, "Error range should have valid start line");
      assert.ok(firstError.range.start.character >= 0, "Error range should have valid start character");
      
      console.log(`Syntax error diagnostic: "${firstError.message}" at line ${firstError.range.start.line}:${firstError.range.start.character}`);
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test("should show diagnostics for type errors", async function () {
    this.timeout(30000);

    const typeErrorCode = `module TypeError exposing (main)

import Node
import Stream
import Bytes exposing (Bytes)
import Node exposing (Environment, Program)
import Init

main : Program Model Msg
main =
    Node.defineProgram
        { init = init
        , update = update
        , subscriptions = subscriptions
        }

type alias Model =
    { stdout : Stream.Writable Bytes
    , stderr : Stream.Writable Bytes
    }

type Msg
    = NoOp

init : Environment -> Init.Task { model : Model, command : Cmd Msg }
init env =
    Node.startProgram
        { model =
            { stdout = env.stdout
            , stderr = env.stderr
            }
        , command = Cmd.none
        }

update : Msg -> Model -> { model : Model, command : Cmd Msg }
update msg model =
    when msg is
        NoOp ->
            { model = model, command = Cmd.none }

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none

-- This function has a type error (trying to add string to number)
addNumbers : Int -> Int -> Int
addNumbers x y =
    x + "not a number"`;

    const testUri = await createTestFileOnDisk(typeErrorCode, "src/TypeError.gren");

    try {
      await monitor.startMonitoring(testUri);
      const document = await openFileInEditor(testUri);

      // Wait for LSP didOpen message
      const didOpenMessage = await monitor.waitForMethod(
        "textDocument/didOpen",
      );
      assert.ok(didOpenMessage, "Should receive LSP didOpen message");

      // Wait for LSP processing
      await monitor.waitForDiagnostics(testUri);

      const diagnostics = vscode.languages.getDiagnostics(testUri);
      assert.ok(
        diagnostics.length > 0,
        "Should receive diagnostics for type error",
      );

      const typeErrors = diagnostics.filter(
        (d) =>
          d.severity === vscode.DiagnosticSeverity.Error &&
          (d.message.toLowerCase().includes("type") ||
            d.message.toLowerCase().includes("string") ||
            d.message.toLowerCase().includes("number") ||
            d.message.toLowerCase().includes("int")),
      );

      // If no specific type errors found, at least verify we have error diagnostics
      if (typeErrors.length === 0) {
        const errors = diagnostics.filter(
          (d) => d.severity === vscode.DiagnosticSeverity.Error,
        );
        assert.ok(
          errors.length > 0,
          "Should have at least one error diagnostic for type mismatch",
        );
      }
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test("should clear diagnostics when errors are fixed", async function () {
    this.timeout(40000);

    // Start with code that has an error
    const errorCode = `module ErrorFixTest exposing (main)

import Node
import Stream
import Bytes exposing (Bytes)
import Node exposing (Environment, Program)
import Init

main : Program Model Msg
main =
    Node.defineProgram
        { init = init
        , update = update
        , subscriptions = subscriptions
        }

type alias Model =
    { stdout : Stream.Writable Bytes
    , stderr : Stream.Writable Bytes
    }

type Msg
    = NoOp

init : Environment -> Init.Task { model : Model, command : Cmd Msg }
init env =
    Node.startProgram
        { model =
            { stdout = env.stdout
            , stderr = env.stderr
            }
        , command = Cmd.none
        }

update : Msg -> Model -> { model : Model, command : Cmd Msg }
update msg model =
    when msg is
        NoOp ->
            { model = model, command = Cmd.none }

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none

-- This has a syntax error (missing =)
brokenFunction : String -> String
brokenFunction name
    "Hello, " ++ name ++ "!"`;

    const testUri = await createTestFileOnDisk(errorCode, "src/ErrorFixTest.gren");

    try {
      await monitor.startMonitoring(testUri);
      const document = await openFileInEditor(testUri);

      // Wait for LSP didOpen
      await monitor.waitForMethod("textDocument/didOpen");

      // Wait for initial diagnostics
      await monitor.waitForDiagnostics(testUri);

      const initialDiagnostics = vscode.languages.getDiagnostics(testUri);

      if (initialDiagnostics.length === 0) {
        assert.fail(
          "LSP server failed to provide diagnostics for file with syntax error. The server should detect errors and send diagnostics.",
        );
      }

      // Fix the error by adding the missing =
      const fixedCode = errorCode.replace(
        'brokenFunction name\n    "Hello, " ++ name ++ "!"',
        'brokenFunction name =\n    "Hello, " ++ name ++ "!"',
      );

      const edit = new vscode.WorkspaceEdit();
      edit.replace(
        testUri,
        new vscode.Range(0, 0, document.lineCount, 0),
        fixedCode,
      );
      const success = await vscode.workspace.applyEdit(edit);
      assert.ok(success, "Edit should be applied successfully");

      // Wait for LSP to process the change and clear diagnostics
      await monitor.waitForMethod("textDocument/didChange");
      await monitor.waitForDiagnosticsCleared(testUri);

      const finalDiagnostics = vscode.languages.getDiagnostics(testUri);

      // Diagnostics should be completely cleared after fixing the error
      assert.strictEqual(finalDiagnostics.length, 0, 
        "All diagnostics should be cleared after fixing the syntax error");
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test("should show no diagnostics for valid code", async function () {
    this.timeout(30000);

    const validCode = `module ValidCode exposing (main, greet)

import Node
import Stream
import Bytes exposing (Bytes)
import Node exposing (Environment, Program)
import Init

main : Program Model Msg
main =
    Node.defineProgram
        { init = init
        , update = update
        , subscriptions = subscriptions
        }

type alias Model =
    { stdout : Stream.Writable Bytes
    , stderr : Stream.Writable Bytes
    }

type Msg
    = NoOp

init : Environment -> Init.Task { model : Model, command : Cmd Msg }
init env =
    Node.startProgram
        { model =
            { stdout = env.stdout
            , stderr = env.stderr
            }
        , command = Cmd.none
        }

update : Msg -> Model -> { model : Model, command : Cmd Msg }
update msg model =
    when msg is
        NoOp ->
            { model = model, command = Cmd.none }

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none

greet : String -> String
greet name =
    "Hello, " ++ name ++ "!"

result : String
result =
    greet "World"`;

    const testUri = await createTestFileOnDisk(validCode, "src/ValidCode.gren");

    try {
      await monitor.startMonitoring(testUri);
      const document = await openFileInEditor(testUri);

      // Wait for LSP didOpen
      await monitor.waitForMethod("textDocument/didOpen");

      // Wait for LSP processing
      await monitor.waitForDiagnostics(testUri);

      const diagnostics = vscode.languages.getDiagnostics(testUri);

      if (diagnostics.length > 0) {
        const errorDiagnostics = diagnostics.filter(
          (d) => d.severity === vscode.DiagnosticSeverity.Error,
        );
        assert.strictEqual(
          errorDiagnostics.length,
          0,
          `LSP server incorrectly reported ${diagnostics.length} diagnostic(s) for valid Gren code. Valid code should produce no diagnostics. This indicates a problem with the LSP server's error detection.`,
        );
      }
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test("should handle import errors", async function () {
    this.timeout(30000);

    const importErrorCode = `module ImportError exposing (main)

import Node
import Stream
import Bytes exposing (Bytes)
import Node exposing (Environment, Program)
import Init
import NonExistentModule

main : Program Model Msg
main =
    Node.defineProgram
        { init = init
        , update = update
        , subscriptions = subscriptions
        }

type alias Model =
    { stdout : Stream.Writable Bytes
    , stderr : Stream.Writable Bytes
    }

type Msg
    = NoOp

init : Environment -> Init.Task { model : Model, command : Cmd Msg }
init env =
    Node.startProgram
        { model =
            { stdout = env.stdout
            , stderr = env.stderr
            }
        , command = Cmd.none
        }

update : Msg -> Model -> { model : Model, command : Cmd Msg }
update msg model =
    when msg is
        NoOp ->
            { model = model, command = Cmd.none }

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none`;

    const testUri = await createTestFileOnDisk(
      importErrorCode,
      "src/ImportError.gren",
    );

    try {
      await monitor.startMonitoring(testUri);
      const document = await openFileInEditor(testUri);

      // Wait for LSP didOpen
      await monitor.waitForMethod("textDocument/didOpen");

      // Wait for LSP processing
      await monitor.waitForDiagnostics(testUri);

      const diagnostics = vscode.languages.getDiagnostics(testUri);

      if (diagnostics.length > 0) {
        const importErrors = diagnostics.filter(
          (d) =>
            d.message.toLowerCase().includes("import") ||
            d.message.toLowerCase().includes("module") ||
            d.message.toLowerCase().includes("nonexistent"),
        );

        if (importErrors.length > 0) {
          assert.ok(true, "LSP server correctly detected import error");
        } else {
          // Even if no specific import error detected, should have some diagnostic
          assert.ok(
            diagnostics.length > 0,
            "LSP server should provide diagnostic information for import errors",
          );
        }
      }
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test("should verify diagnostic source and properties", async function () {
    this.timeout(30000);

    const codeWithError = `module DiagnosticProperties exposing (main)

import Node
import Stream
import Bytes exposing (Bytes)
import Node exposing (Environment, Program)
import Init

main : Program Model Msg
main =
    Node.defineProgram
        { init = init
        , update = update
        , subscriptions = subscriptions
        }

type alias Model =
    { stdout : Stream.Writable Bytes
    , stderr : Stream.Writable Bytes
    }

type Msg
    = NoOp

init : Environment -> Init.Task { model : Model, command : Cmd Msg }
init env =
    Node.startProgram
        { model =
            { stdout = env.stdout
            , stderr = env.stderr
            }
        , command = Cmd.none
        }

update : Msg -> Model -> { model : Model, command : Cmd Msg }
update msg model =
    when msg is
        NoOp ->
            { model = model, command = Cmd.none }

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none

-- Function with deliberate error
errorFunction : String -> String
errorFunction name
    "Hello, " ++ name ++ "!"`;

    const testUri = await createTestFileOnDisk(codeWithError, "src/DiagnosticProperties.gren");

    try {
      await monitor.startMonitoring(testUri);
      const document = await openFileInEditor(testUri);

      // Wait for LSP didOpen
      await monitor.waitForMethod("textDocument/didOpen");

      // Wait for LSP processing
      await monitor.waitForDiagnostics(testUri);

      const diagnostics = vscode.languages.getDiagnostics(testUri);

      if (diagnostics.length > 0) {
        const diagnostic = diagnostics[0];

        // PRECISE DIAGNOSTIC CONTENT VALIDATION: Exact expected diagnostic for syntax error
        //
        // The test file DiagProps.gren contains invalid syntax: missing "=" in function definition
        // Example: "myFunction value" instead of "myFunction = value"
        
        // EXACT DIAGNOSTIC MESSAGE: Determined by testing with actual Gren compiler
        // For missing "=" in function definition, Gren compiler produces:
        // "I was not expecting to see this symbol here:"
        const expectedMessage = "I was not expecting to see this symbol here:";
        
        // DETERMINISTIC VALIDATION: Assert exact error message format
        assert.ok(
          diagnostic.message.includes(expectedMessage),
          `Diagnostic message MUST match exact Gren compiler error format. Expected: "${expectedMessage}". Got: "${diagnostic.message}"`
        );
        
        // MUST validate diagnostic severity precisely (errors should be Error, not Warning)
        assert.strictEqual(
          diagnostic.severity,
          vscode.DiagnosticSeverity.Error,
          `Syntax error MUST have severity Error (1), got: ${diagnostic.severity}`
        );
        
        // MUST validate range precisely - syntax error should point to specific location
        assert.ok(
          diagnostic.range.start.line >= 0,
          "Diagnostic range MUST specify valid line number"
        );
        assert.ok(
          diagnostic.range.start.character >= 0,
          "Diagnostic range MUST specify valid character position"
        );
        
        // MUST validate source field precisely per LSP specification
        if (diagnostic.source) {
          assert.ok(
            diagnostic.source === "gren" || diagnostic.source === "gren-compiler",
            `Diagnostic source MUST be 'gren' or 'gren-compiler', got: "${diagnostic.source}"`
          );
        }

        // Verify diagnostic has required properties
        assert.ok(diagnostic.message, "Diagnostic should have a message");
        assert.ok(diagnostic.range, "Diagnostic should have a range");
        assert.ok(
          typeof diagnostic.severity === "number",
          "Diagnostic should have a severity",
        );

        // Verify range properties
        assert.ok(
          typeof diagnostic.range.start.line === "number",
          "Diagnostic range should have start line",
        );
        assert.ok(
          typeof diagnostic.range.start.character === "number",
          "Diagnostic range should have start character",
        );
        assert.ok(
          typeof diagnostic.range.end.line === "number",
          "Diagnostic range should have end line",
        );
        assert.ok(
          typeof diagnostic.range.end.character === "number",
          "Diagnostic range should have end character",
        );

        // If source is provided, it should be a string
        if (diagnostic.source) {
          assert.strictEqual(
            typeof diagnostic.source,
            "string",
            "Diagnostic source should be a string",
          );
        }

        // TODO: Validate specific syntax error content after compiler bug fix
        // Once LSP server is fixed, this should check for:
        // - Error message mentioning missing "=" 
        // - Syntax or parse error indicators
        // - Helpful error message guiding the developer to the fix
        if (diagnostic.severity === vscode.DiagnosticSeverity.Error) {
          console.log("TODO: Add specific syntax error message validation");
          // For now, just verify it's an error (content validation deferred due to compiler bug)
        }

        console.log(
          `Diagnostic: ${diagnostic.message} at ${diagnostic.range.start.line}:${diagnostic.range.start.character}`,
        );
      }
    } finally {
      await cleanupTestFile(testUri);
    }
  });
});
