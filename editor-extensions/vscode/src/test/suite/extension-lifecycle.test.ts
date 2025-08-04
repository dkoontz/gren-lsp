import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { before, after, beforeEach, afterEach } from 'mocha';
import { ObservedLSPMessageMonitor, createTestFileOnDisk, cleanupTestFile, closeAllFiles, openFileInEditor } from './helpers/lsp-message-helper';
import { testLogger } from './helpers/test-logger';
import { State } from 'vscode-languageclient/node';

suite('Extension Lifecycle & Server Management Tests', () => {
  let monitor: ObservedLSPMessageMonitor;
  let testWorkspaceUri: vscode.Uri;
  let serverPath: string;

  before(async function() {
    this.timeout(30000);
    
    // Find the LSP server binary
    const possiblePaths = [
      path.join(__dirname, '..', '..', '..', '..', '..', 'lsp-server', 'target', 'debug', 'gren-lsp'),
      path.join(__dirname, '..', '..', '..', '..', '..', '..', 'lsp-server', 'target', 'debug', 'gren-lsp'),
      '/Users/david/dev/gren-lsp/lsp-server/target/debug/gren-lsp',
      // Legacy paths for backwards compatibility
      path.join(__dirname, '..', '..', '..', '..', '..', 'target', 'debug', 'gren-lsp'),
      path.join(__dirname, '..', '..', '..', '..', '..', '..', 'target', 'debug', 'gren-lsp'),
      '/Users/david/dev/gren-lsp/target/debug/gren-lsp'
    ];

    for (const candidatePath of possiblePaths) {
      if (fs.existsSync(candidatePath)) {
        serverPath = candidatePath;
        break;
      }
    }

    if (!serverPath) {
      throw new Error('LSP server binary not found. Please run `just build` from the project root.');
    }

    const workspaceFolders = vscode.workspace.workspaceFolders;
    assert.ok(workspaceFolders && workspaceFolders.length > 0, 'Test workspace should be open');
    testWorkspaceUri = workspaceFolders[0].uri;

    monitor = new ObservedLSPMessageMonitor();
    await monitor.preActivateExtension();
    await monitor.attachToClient();
    
    testLogger.verbose('🎯 Extension lifecycle test setup complete');
  });

  after(async function() {
    if (monitor) {
      monitor.dispose();
    }
  });

  beforeEach(async () => {
    await closeAllFiles();
    monitor.clear();
  });

  afterEach(async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    
    // Give a brief moment for any pending operations to settle before stopping monitoring
    await new Promise(resolve => setTimeout(resolve, 500));
    
    monitor.stopMonitoring();
  });

  test('LSP server binary should exist', () => {
    assert.ok(fs.existsSync(serverPath), `LSP server binary should exist at ${serverPath}`);
  });

  test('LSP server binary should be executable', () => {
    try {
      fs.accessSync(serverPath, fs.constants.F_OK | fs.constants.X_OK);
      assert.ok(true, 'LSP server binary is executable');
    } catch (err) {
      assert.fail(`LSP server binary is not executable: ${err}`);
    }
  });

  test('LSP server should respond to --help', async function() {
    this.timeout(10000);
    
    const { spawn } = require('child_process');
    
    return new Promise<void>((resolve, reject) => {
      const process = spawn(serverPath, ['--help']);
      let output = '';
      let errorOutput = '';

      process.stdout.on('data', (data: Buffer) => {
        output += data.toString();
      });

      process.stderr.on('data', (data: Buffer) => {
        errorOutput += data.toString();
      });

      process.on('close', (code: number) => {
        if (code === 0 || code === null) {
          // Server should respond with help text
          assert.ok(output.length > 0 || errorOutput.length > 0, 'Server should produce help output');
          resolve();
        } else {
          reject(new Error(`Server exited with code ${code}. Output: ${output}. Error: ${errorOutput}`));
        }
      });

      process.on('error', (err: Error) => {
        reject(new Error(`Failed to spawn server process: ${err.message}`));
      });

      // Timeout after 5 seconds
      setTimeout(() => {
        process.kill();
        reject(new Error('Server help command timed out'));
      }, 5000);
    });
  });

  test('extension should be installed and discoverable', async function() {
    this.timeout(10000);

    // Check if extension is installed
    const extension = vscode.extensions.getExtension('gren-lsp.gren-lsp');
    assert.ok(extension, 'Gren LSP extension should be installed');
    assert.ok(extension.isActive, 'Extension should be active after pre-activation');
  });

  test('extension should activate on Gren file', async function() {
    this.timeout(15000);

    const simpleCode = `module Simple exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }`;

    const testUri = await createTestFileOnDisk(simpleCode, 'lifecycle-test-activation.gren');
    
    try {
      // Verify extension is found and can be activated
      const extension = vscode.extensions.getExtension('gren-lsp.gren-lsp');
      assert.ok(extension, 'Gren LSP extension should be installed');
      
      // Start monitoring
      await monitor.startMonitoring(testUri);
      
      // After successful monitoring setup, extension should be active
      assert.ok(extension.isActive, 'Extension should be in active state');
      
      // Open the document - this should work with active extension
      const document = await openFileInEditor(testUri);
      assert.strictEqual(document.languageId, 'gren', 'Document should be identified as Gren');
      
      // Verify LSP communication is working by checking for didOpen
      const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
      assert.ok(didOpenMessage, 'Opening Gren file should trigger LSP communication');
      
      // Verify basic document functionality
      assert.ok(document.getText().includes('Node.Program'), 'Document should contain Gren code');
      
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test('extension configuration should have valid default values', async function() {
    this.timeout(10000);

    const config = vscode.workspace.getConfiguration('grenLsp');
    
    // Verify default configuration values
    const traceLevel = config.get<string>('trace.server', 'off');
    assert.ok(['off', 'messages', 'verbose'].includes(traceLevel), 
      `Trace level should be valid: ${traceLevel}`);

    const autoDownload = config.get<boolean>('compiler.autoDownload', true);
    assert.ok(typeof autoDownload === 'boolean', 'Auto download should be boolean');
  });

  test('should handle server process failure gracefully', async function() {
    this.timeout(30000);
    
    testLogger.verbose('🧪 Starting server failure test...');
    
    const testCode = `module ServerFailureTest exposing (main, greet)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }

greet : String -> String
greet name =
    "Hello, " ++ name ++ "!"

useGreet : String
useGreet =
    greet "World"`;

    // 1. Baseline: Verify normal LSP operation
    testLogger.verbose('📝 Step 1: Testing baseline LSP functionality...');
    const testUri = await createTestFileOnDisk(testCode, 'lifecycle-server-failure-test.gren');
    
    try {
      await monitor.startMonitoring(testUri);
      const document = await openFileInEditor(testUri);
      
      // Verify normal didOpen works
      const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
      assert.ok(didOpenMessage, 'Initial LSP communication should work');
      testLogger.verbose('✅ Baseline LSP communication working');

      // Make a small edit to verify didChange works
      const edit = new vscode.WorkspaceEdit();
      edit.insert(testUri, new vscode.Position(0, 0), '-- Initial comment\\n');
      const editSuccess = await vscode.workspace.applyEdit(edit);
      assert.ok(editSuccess, 'Edit should be applied successfully');

      const didChangeMessage = await monitor.waitForMethod('textDocument/didChange');
      assert.ok(didChangeMessage, 'Normal didChange should work before killing server');
      testLogger.verbose('✅ Normal edit operations working');

      // 2. Kill server process
      testLogger.verbose('🔪 Step 2: Killing LSP server process...');
      await monitor.killServerProcess();

      // 3. Wait for client state to change to Stopped
      testLogger.verbose('⏳ Step 3: Waiting for client state to change to Stopped...');
      await monitor.waitForClientStateChange(State.Stopped, 15000);
      testLogger.verbose('✅ Client state changed to Stopped');
      
      // Give the client additional time to process the failure and clean up pending operations
      testLogger.verbose('⏳ Allowing client to clean up pending operations...');
      await new Promise(resolve => setTimeout(resolve, 2000));
      testLogger.verbose('✅ Client cleanup period completed');

      // 4. Attempt operation and verify it fails/times out
      testLogger.verbose('📝 Step 4: Testing LSP behavior after server kill...');
      
      // Clear previous messages to isolate the failure test
      monitor.clear();
      
      const failureEdit = new vscode.WorkspaceEdit();
      failureEdit.insert(testUri, new vscode.Position(1, 0), '-- Server killed test\\n');
      const failureEditSuccess = await vscode.workspace.applyEdit(failureEdit);
      assert.ok(failureEditSuccess, 'Edit should still be applied to document');

      // This should timeout since server is dead
      testLogger.verbose('⏳ Expecting timeout for LSP message (server is dead)...');
      try {
        await monitor.waitForMethod('textDocument/didChange', 3000); // Short timeout
        assert.fail('Expected timeout when server is killed, but LSP message was received');
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        assert.ok(errorMessage.includes('Timeout'), `Should timeout waiting for LSP response. Got: ${errorMessage}`);
        testLogger.verbose('✅ Confirmed LSP timeout after server kill');
      }

      // 5. Server failure test complete
      testLogger.verbose('✅ Server failure test completed successfully');
      
    } finally {
      // Clean up test file
      await cleanupTestFile(testUri);
      
      // Reset extension state for subsequent tests
      try {
        testLogger.verbose('🔄 Resetting extension state after server failure test...');
        
        // Create new monitor and pre-activate extension to reset state
        const recoveryMonitor = new ObservedLSPMessageMonitor();
        await recoveryMonitor.preActivateExtension();
        await recoveryMonitor.attachToClient();
        
        testLogger.verbose('✅ Extension state reset complete');
        recoveryMonitor.dispose();
        
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        testLogger.verbose(`⚠️ Extension reset failed: ${errorMessage}. Subsequent tests may need fresh VS Code instance.`);
      }
    }
  });
});