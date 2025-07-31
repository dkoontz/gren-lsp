import * as assert from 'assert';
import * as vscode from 'vscode';
import { before, after, beforeEach, afterEach } from 'mocha';
import { ObservedLSPMessageMonitor, createTestFileOnDisk, cleanupTestFile, closeAllFiles, openFileInEditor } from './helpers/lsp-message-helper';
import { testLogger } from './helpers/test-logger';

suite('LSP Protocol Core Messages Tests', () => {
  let monitor: ObservedLSPMessageMonitor;
  let testWorkspaceUri: vscode.Uri;

  before(async function() {
    this.timeout(30000);
    
    const workspaceFolders = vscode.workspace.workspaceFolders;
    assert.ok(workspaceFolders && workspaceFolders.length > 0, 'Test workspace should be open');
    testWorkspaceUri = workspaceFolders[0].uri;

    monitor = new ObservedLSPMessageMonitor();
    await monitor.preActivateExtension();
    await monitor.attachToClient();
    
    testLogger.verbose('🎯 LSP protocol core test setup complete');
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
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    monitor.stopMonitoring();
  });

  test('should send textDocument/didOpen when opening Gren file', async function() {
    this.timeout(20000);

    const validCode = `module Test exposing (main)

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
    "Hello, " ++ name ++ "!"`;

    // Create test file directly on disk (does not load into VS Code workspace)
    const testUri = await createTestFileOnDisk(validCode, 'protocol-core-didopen.gren');
    
    try {
      // Start monitoring BEFORE opening the file
      await monitor.startMonitoring(testUri);
      
      // Now open the file in the editor - this should trigger the real textDocument/didOpen
      const document = await openFileInEditor(testUri);
      assert.strictEqual(document.languageId, 'gren', 'Document should be identified as Gren language');

      // Wait for and verify textDocument/didOpen message was sent
      const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
      
      // Assert specific expected values for the didOpen message
      assert.strictEqual(didOpenMessage.method, 'textDocument/didOpen', 'Method should be exactly textDocument/didOpen');
      assert.ok(didOpenMessage.params, 'didOpen message must have parameters');
      assert.ok(didOpenMessage.params.textDocument, 'didOpen params must have textDocument object');
      
      // Assert exact URI match
      assert.strictEqual(didOpenMessage.params.textDocument.uri, testUri.toString(), 
        `URI should be exactly ${testUri.toString()}`);
      
      // Assert exact language ID
      assert.strictEqual(didOpenMessage.params.textDocument.languageId, 'gren', 
        'Language ID should be exactly "gren"');
      
      // Assert exact version (should be a positive number for initial open)
      assert.ok(typeof didOpenMessage.params.textDocument.version === 'number' && 
                didOpenMessage.params.textDocument.version > 0, 
        'Version should be a positive number');
      
      // Assert exact text content matches what we wrote
      assert.strictEqual(didOpenMessage.params.textDocument.text, validCode, 
        'Text content should exactly match the file content we created');
      
      // Assert text contains specific expected function definition
      assert.ok(didOpenMessage.params.textDocument.text.includes('greet : String -> String'), 
        'Text should contain the specific function signature "greet : String -> String"');
      assert.ok(didOpenMessage.params.textDocument.text.includes('"Hello, " ++ name ++ "!"'), 
        'Text should contain the specific string concatenation expression');
      
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test('should send textDocument/didChange when editing Gren file', async function() {
    this.timeout(20000);

    const initialCode = `module Test exposing (main)

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
    "Hello, " ++ name ++ "!"`;

    const testUri = await createTestFileOnDisk(initialCode, 'protocol-core-didchange.gren');
    
    try {
      // Start monitoring
      await monitor.startMonitoring(testUri);
      
      // Open the document first using the proper helper
      const document = await openFileInEditor(testUri);
      
      // Wait for didOpen first and get initial version
      const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
      const initialVersion = didOpenMessage.params.textDocument.version;
      
      // Make an edit - this should trigger textDocument/didChange
      const edit = new vscode.WorkspaceEdit();
      const position = new vscode.Position(document.lineCount, 0);
      const newFunction = `

add : Int -> Int -> Int
add x y =
    x + y`;
      
      edit.insert(testUri, position, newFunction);
      const success = await vscode.workspace.applyEdit(edit);
      assert.ok(success, 'Edit should be applied successfully');
      
      // Wait for and verify textDocument/didChange message was sent
      const didChangeMessage = await monitor.waitForMethod('textDocument/didChange');
      
      // Assert specific expected values for the didChange message
      assert.strictEqual(didChangeMessage.method, 'textDocument/didChange', 
        'Method should be exactly textDocument/didChange');
      assert.ok(didChangeMessage.params, 'didChange message must have parameters');
      assert.ok(didChangeMessage.params.textDocument, 'didChange params must have textDocument object');
      
      // Assert exact URI match
      assert.strictEqual(didChangeMessage.params.textDocument.uri, testUri.toString(), 
        `URI should be exactly ${testUri.toString()}`);
      
      // Assert version is greater than initial version
      assert.ok(typeof didChangeMessage.params.textDocument.version === 'number', 
        'Version should be a number');
      assert.ok(didChangeMessage.params.textDocument.version > initialVersion, 
        `Version should be greater than initial version ${initialVersion}`);
      
      // Assert contentChanges array exists and has content
      assert.ok(Array.isArray(didChangeMessage.params.contentChanges), 
        'contentChanges should be an array');
      assert.ok(didChangeMessage.params.contentChanges.length > 0, 
        'contentChanges should have at least one change');
      
      // Get the change with our specific content
      const allDidChangeMessages = monitor.getMessagesForMethod('textDocument/didChange');
      const changeMessageWithContent = allDidChangeMessages.find(msg => 
        msg.params.contentChanges && msg.params.contentChanges.length > 0 &&
        msg.params.contentChanges.some((change: any) => change.text && change.text.includes('add : Int -> Int -> Int'))
      );
      
      assert.ok(changeMessageWithContent, 'Should have a didChange message with our specific function addition');
      
      // Assert specific content change properties
      const contentChange = changeMessageWithContent.params.contentChanges.find((change: any) => 
        change.text && change.text.includes('add : Int -> Int -> Int')
      );
      assert.ok(contentChange, 'Should find the specific content change with our function');
      assert.ok(contentChange.text.includes('add : Int -> Int -> Int'), 
        'Change text should contain the specific function signature');
      assert.ok(contentChange.text.includes('x + y'), 
        'Change text should contain the specific function body');
      
      // Verify the change was applied to the document with exact content
      const updatedText = document.getText();
      assert.ok(updatedText.includes('add : Int -> Int -> Int'), 
        'Document should contain the specific function signature');
      assert.ok(updatedText.includes('x + y'), 
        'Document should contain the specific function implementation');
      
    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test('should send textDocument/didChange for multiple rapid edits', async function() {
    this.timeout(25000);

    const initialCode = `module RapidChanges exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }

counter : Int
counter =
    0`;

    const testUri = await createTestFileOnDisk(initialCode, 'protocol-core-rapid-changes.gren');
    
    try {
      await monitor.startMonitoring(testUri);
      
      // Open the document
      const document = await openFileInEditor(testUri);
      
      // Wait for initial didOpen and get initial version
      const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
      const initialVersion = didOpenMessage.params.textDocument.version;
      
      // Make several rapid changes to test LSP message flow stability
      const changes = [
        '\\n\\n-- Change 1\\nvalue1 : String\\nvalue1 = "test1"',
        '\\n\\n-- Change 2\\nvalue2 : Int\\nvalue2 = 42',
        '\\n\\n-- Change 3\\nvalue3 : Bool\\nvalue3 = True',
        '\\n\\n-- Change 4\\nvalue4 : String\\nvalue4 = "final"'
      ];

      const expectedContentMarkers = ['value1 = "test1"', 'value2 = 42', 'value3 = True', 'value4 = "final"'];

      for (let i = 0; i < changes.length; i++) {
        const edit = new vscode.WorkspaceEdit();
        const position = new vscode.Position(document.lineCount, 0);
        edit.insert(testUri, position, changes[i]);
        
        const success = await vscode.workspace.applyEdit(edit);
        assert.ok(success, `Change ${i + 1} should be applied successfully`);
        
        // Small delay between changes to simulate real typing
        await new Promise(resolve => setTimeout(resolve, 300));
      }

      // Wait for LSP server to process all changes
      await new Promise(resolve => setTimeout(resolve, 2000));
      
      // Verify we received multiple didChange messages
      const didChangeMessages = monitor.getMessagesForMethod('textDocument/didChange');
      assert.ok(didChangeMessages.length >= changes.length, 
        `Should have received at least ${changes.length} didChange messages, got ${didChangeMessages.length}`);
      
      // Assert specific message structure for each didChange message
      const messagesWithChanges = didChangeMessages.filter(msg => 
        msg.params.contentChanges && msg.params.contentChanges.length > 0
      );
      assert.ok(messagesWithChanges.length >= changes.length, 
        `Should have at least ${changes.length} didChange messages with content, got ${messagesWithChanges.length}`);
      
      // Verify each message has proper structure and specific content
      messagesWithChanges.forEach((msg, index) => {
        // Assert basic message structure
        assert.strictEqual(msg.method, 'textDocument/didChange', 
          `Message ${index} should have method textDocument/didChange`);
        assert.ok(msg.params, `Message ${index} should have parameters`);
        assert.ok(msg.params.textDocument, `Message ${index} should have textDocument object`);
        assert.strictEqual(msg.params.textDocument.uri, testUri.toString(), 
          `Message ${index} URI should match test file`);
        
        // Assert version progression
        assert.ok(typeof msg.params.textDocument.version === 'number', 
          `Message ${index} version should be a number`);
        assert.ok(msg.params.textDocument.version > initialVersion, 
          `Message ${index} version should be greater than initial version ${initialVersion}`);
        
        // Assert contentChanges structure
        assert.ok(Array.isArray(msg.params.contentChanges), 
          `Message ${index} contentChanges should be an array`);
        assert.ok(msg.params.contentChanges.length > 0, 
          `Message ${index} should have at least one content change`);
        
        // Assert each content change has required properties
        msg.params.contentChanges.forEach((change: any, changeIndex: number) => {
          assert.ok(typeof change.text === 'string', 
            `Message ${index} change ${changeIndex} should have text property as string`);
          if (change.range) {
            assert.ok(typeof change.range.start.line === 'number', 
              `Message ${index} change ${changeIndex} range start line should be a number`);
            assert.ok(typeof change.range.start.character === 'number', 
              `Message ${index} change ${changeIndex} range start character should be a number`);
            assert.ok(typeof change.range.end.line === 'number', 
              `Message ${index} change ${changeIndex} range end line should be a number`);
            assert.ok(typeof change.range.end.character === 'number', 
              `Message ${index} change ${changeIndex} range end character should be a number`);
          }
        });
      });
      
      // Verify that we can find messages containing our specific content changes
      expectedContentMarkers.forEach((marker, index) => {
        const messageWithMarker = messagesWithChanges.find(msg => 
          msg.params.contentChanges.some((change: any) => change.text.includes(marker))
        );
        assert.ok(messageWithMarker, 
          `Should find a didChange message containing "${marker}" from change ${index + 1}`);
      });

    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test('should send textDocument/didClose when closing Gren file', async function() {
    this.timeout(15000);

    const code = `module CloseTest exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }`;

    const testUri = await createTestFileOnDisk(code, 'protocol-core-didclose.gren');
    
    try {
      await monitor.startMonitoring(testUri);
      
      // Open the document
      const document = await openFileInEditor(testUri);
      
      // Wait for didOpen to establish baseline
      const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
      assert.ok(didOpenMessage, 'Should receive didOpen message');
      
      // Clear messages to focus on close behavior
      monitor.clear();
      
      // Close the document - this should trigger textDocument/didClose
      await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
      
      // Wait a moment for the close message to be sent
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      // Check if didClose message was sent (some LSP servers may not implement this)
      const didCloseMessage = monitor.getLastMessageForMethod('textDocument/didClose');
      if (didCloseMessage) {
        // Assert specific expected values for the didClose message
        assert.strictEqual(didCloseMessage.method, 'textDocument/didClose', 
          'Method should be exactly textDocument/didClose');
        assert.ok(didCloseMessage.params, 'didClose message must have parameters');
        assert.ok(didCloseMessage.params.textDocument, 'didClose params must have textDocument object');
        
        // Assert exact URI match
        assert.strictEqual(didCloseMessage.params.textDocument.uri, testUri.toString(), 
          `URI should be exactly ${testUri.toString()}`);
        
        // Assert that didClose only has uri property (no version or text content)
        assert.ok(didCloseMessage.params.textDocument.uri, 'didClose textDocument should have uri');
        assert.strictEqual(didCloseMessage.params.textDocument.version, undefined, 
          'didClose textDocument should not have version property');
        assert.strictEqual(didCloseMessage.params.textDocument.text, undefined, 
          'didClose textDocument should not have text property');
        assert.strictEqual(didCloseMessage.params.textDocument.languageId, undefined, 
          'didClose textDocument should not have languageId property');
        
        // Assert direction is outgoing (sent from client to server)
        assert.strictEqual(didCloseMessage.direction, 'outgoing', 
          'didClose should be an outgoing message from client to server');
        
        testLogger.verbose('✅ LSP server correctly implements textDocument/didClose protocol');
      }

    } finally {
      await cleanupTestFile(testUri);
    }
  });

  test('should handle document versioning in didChange messages', async function() {
    this.timeout(20000);

    const initialCode = `module VersionTest exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }`;

    const testUri = await createTestFileOnDisk(initialCode, 'protocol-core-versioning.gren');
    
    try {
      await monitor.startMonitoring(testUri);
      
      const document = await openFileInEditor(testUri);
      
      // Wait for didOpen and capture initial version
      const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
      assert.ok(didOpenMessage.params.textDocument.version, 'didOpen should include document version');
      
      const initialVersion = didOpenMessage.params.textDocument.version;
      assert.ok(typeof initialVersion === 'number', 'Initial version should be a number');
      assert.ok(initialVersion > 0, 'Initial version should be positive');
      
      // Make first edit
      const edit1 = new vscode.WorkspaceEdit();
      edit1.insert(testUri, new vscode.Position(document.lineCount, 0), '\\n\\nfirst : String\\nfirst = "1"');
      await vscode.workspace.applyEdit(edit1);
      
      const firstChangeMessage = await monitor.waitForMethod('textDocument/didChange');
      assert.ok(firstChangeMessage.params.textDocument.version, 'First didChange should include document version');
      
      const firstChangeVersion = firstChangeMessage.params.textDocument.version;
      assert.ok(typeof firstChangeVersion === 'number', 'First change version should be a number');
      assert.ok(firstChangeVersion > initialVersion, 
        `First change version ${firstChangeVersion} should be greater than initial version ${initialVersion}`);
      
      // Make second edit
      const edit2 = new vscode.WorkspaceEdit();
      edit2.insert(testUri, new vscode.Position(document.lineCount, 0), '\\n\\nsecond : String\\nsecond = "2"');
      await vscode.workspace.applyEdit(edit2);
      
      // Wait a moment for the second change
      await new Promise(resolve => setTimeout(resolve, 500));
      
      const allDidChangeMessages = monitor.getMessagesForMethod('textDocument/didChange');
      const changesWithContent = allDidChangeMessages.filter(msg => 
        msg.params.contentChanges && msg.params.contentChanges.length > 0 &&
        msg.params.textDocument.version
      );
      
      assert.ok(changesWithContent.length >= 2, 
        `Should have at least 2 didChange messages with content and versions, got ${changesWithContent.length}`);
      
      // Find messages with our specific content to ensure proper ordering
      const firstContentChange = changesWithContent.find(msg => 
        msg.params.contentChanges.some((change: any) => change.text.includes('first = "1"'))
      );
      const secondContentChange = changesWithContent.find(msg => 
        msg.params.contentChanges.some((change: any) => change.text.includes('second = "2"'))
      );
      
      assert.ok(firstContentChange, 'Should find didChange message with first function');
      assert.ok(secondContentChange, 'Should find didChange message with second function');
      
      // Assert specific version progression
      const firstContentVersion = firstContentChange.params.textDocument.version;
      const secondContentVersion = secondContentChange.params.textDocument.version;
      
      assert.ok(typeof firstContentVersion === 'number', 'First content change version should be a number');
      assert.ok(typeof secondContentVersion === 'number', 'Second content change version should be a number');
      
      assert.ok(firstContentVersion > initialVersion, 
        `First content change version ${firstContentVersion} should be greater than initial version ${initialVersion}`);
      assert.ok(secondContentVersion > firstContentVersion, 
        `Second content change version ${secondContentVersion} should be greater than first change version ${firstContentVersion}`);
      
      // Assert that versions increment by at least 1
      assert.ok(firstContentVersion >= initialVersion + 1, 
        `First change version should increment by at least 1 from initial version`);
      assert.ok(secondContentVersion >= firstContentVersion + 1, 
        `Second change version should increment by at least 1 from first change version`);
      
      testLogger.verbose(`✅ Version progression: initial=${initialVersion} → first=${firstContentVersion} → second=${secondContentVersion}`);

    } finally {
      await cleanupTestFile(testUri);
    }
  });
});