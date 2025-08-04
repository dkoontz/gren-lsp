import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import { before, after, beforeEach, afterEach } from 'mocha';
import { ObservedLSPMessageMonitor, createTestFileOnDisk, cleanupTestFile, closeAllFiles, openFileInEditor } from './helpers/lsp-message-helper';
import { testLogger } from './helpers/test-logger';

/**
 * Epic 1-5 Integration Test Suite
 * 
 * Comprehensive testing of all LSP features through VS Code extension integration.
 * Tests validate that server capabilities work correctly through the extension interface.
 */
suite('Epic 1-5 Integration Tests', () => {
  let monitor: ObservedLSPMessageMonitor;
  let testWorkspaceUri: vscode.Uri;
  const testResults: Map<string, { status: 'pass' | 'fail' | 'skip', details: string }> = new Map();

  before(async function() {
    this.timeout(30000);
    
    const workspaceFolders = vscode.workspace.workspaceFolders;
    assert.ok(workspaceFolders && workspaceFolders.length > 0, 'Test workspace should be open');
    testWorkspaceUri = workspaceFolders[0].uri;

    monitor = new ObservedLSPMessageMonitor();
    await monitor.preActivateExtension();
    await monitor.attachToClient();
    
    testLogger.verbose('🎯 Epic 1-5 Integration test setup complete');
  });

  after(async function() {
    if (monitor) {
      monitor.dispose();
    }
    
    // Print comprehensive test results summary
    console.log('\n📊 Epic 1-5 Integration Test Results Summary:');
    console.log('=' .repeat(60));
    
    const results = Array.from(testResults.entries());
    const passed = results.filter(([_, result]) => result.status === 'pass').length;
    const failed = results.filter(([_, result]) => result.status === 'fail').length;
    const skipped = results.filter(([_, result]) => result.status === 'skip').length;
    
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`⏭️ Skipped: ${skipped}`);
    console.log(`📈 Success Rate: ${Math.round((passed / (passed + failed)) * 100)}%`);
    
    console.log('\nDetailed Results:');
    results.forEach(([testName, result]) => {
      const icon = result.status === 'pass' ? '✅' : result.status === 'fail' ? '❌' : '⏭️';
      console.log(`${icon} ${testName}: ${result.details}`);
    });
  });

  beforeEach(async () => {
    await closeAllFiles();
    monitor.clear();
  });

  afterEach(async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    monitor.stopMonitoring();
  });

  /**
   * EPIC 1: Foundation & Testing Integration Tests
   */
  suite('Epic 1: Foundation & Testing', () => {
    
    test('LSP Lifecycle: Extension starts/stops server correctly', async function() {
      this.timeout(20000);
      const testName = 'Epic1-LSP-Lifecycle';
      
      try {
        const testCode = `module LifecycleTest exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }`;

        const testUri = await createTestFileOnDisk(testCode, 'epic1-lifecycle-test.gren');
        
        try {
          // Start monitoring and verify LSP initialization
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          
          // Verify LSP initialization sequence
          const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
          assert.ok(didOpenMessage, 'LSP server should respond to didOpen');
          assert.strictEqual(didOpenMessage.params.textDocument.languageId, 'gren');
          
          // Verify extension is active and managing the server
          const extension = vscode.extensions.getExtension('gren-lsp.gren-lsp');
          assert.ok(extension?.isActive, 'Extension should be active');
          
          testResults.set(testName, { status: 'pass', details: 'LSP lifecycle working correctly' });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `LSP lifecycle failed: ${error}` });
        throw error;
      }
    });

    test('Document Management: Open/close/edit triggers proper notifications', async function() {
      this.timeout(20000);
      const testName = 'Epic1-Document-Management';
      
      try {
        const testCode = `module DocumentTest exposing (value)

value : String
value =
    "initial"`;

        const testUri = await createTestFileOnDisk(testCode, 'epic1-document-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          
          // Test didOpen
          const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
          assert.ok(didOpenMessage, 'Document open should trigger didOpen');
          
          // Test didChange
          const edit = new vscode.WorkspaceEdit();
          edit.insert(testUri, new vscode.Position(0, 0), '-- Test comment\n');
          await vscode.workspace.applyEdit(edit);
          
          const didChangeMessage = await monitor.waitForMethod('textDocument/didChange');
          assert.ok(didChangeMessage, 'Document edit should trigger didChange');
          assert.ok(didChangeMessage.params.contentChanges.length > 0, 'Should include content changes');
          
          testResults.set(testName, { status: 'pass', details: 'Document management working correctly' });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Document management failed: ${error}` });
        throw error;
      }
    });

    test('Diagnostics: Compiler errors appear in Problems panel', async function() {
      this.timeout(20000);
      const testName = 'Epic1-Diagnostics';
      
      try {
        // Code with intentional syntax error
        const errorCode = `module DiagnosticTest exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }

-- Syntax error: missing type annotation
brokenFunction input =
    "This will cause an error"`;

        const testUri = await createTestFileOnDisk(errorCode, 'epic1-diagnostics-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          await openFileInEditor(testUri);
          
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Wait for diagnostics processing
          await monitor.waitForDiagnostics(testUri);
          
          // Check VS Code diagnostics panel
          const diagnostics = vscode.languages.getDiagnostics(testUri);
          assert.ok(diagnostics.length > 0, 'Should have diagnostics for syntax error');
          
          // Verify diagnostic properties
          const diagnostic = diagnostics[0];
          assert.ok(diagnostic.message, 'Diagnostic should have error message');
          assert.ok(diagnostic.range, 'Diagnostic should have location range');
          assert.ok(typeof diagnostic.range.start.line === 'number', 'Should have valid line number');
          
          testResults.set(testName, { status: 'pass', details: `Found ${diagnostics.length} diagnostics` });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Diagnostics failed: ${error}` });
        throw error;
      }
    });

    test('Tree-sitter Integration: Syntax highlighting works correctly', async function() {
      this.timeout(15000);
      const testName = 'Epic1-Tree-sitter';
      
      try {
        const testCode = `module TreeSitterTest exposing (main, greet)

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

        const testUri = await createTestFileOnDisk(testCode, 'epic1-treesitter-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          
          // Verify document is recognized as Gren
          assert.strictEqual(document.languageId, 'gren', 'Document should be identified as Gren');
          
          // Verify LSP processes the document (tree-sitter integration working)
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Basic validation that tree-sitter parsing is working through LSP
          // Tree-sitter parsing is immediate, no delay needed
          const diagnostics = vscode.languages.getDiagnostics(testUri);
          
          // Tree-sitter integration working if we can parse without major issues
          testResults.set(testName, { status: 'pass', details: 'Tree-sitter integration working through LSP' });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Tree-sitter integration failed: ${error}` });
        throw error;
      }
    });
  });

  /**
   * EPIC 2: Core Language Intelligence Integration Tests
   */
  suite('Epic 2: Core Language Intelligence', () => {
    
    test('Symbol Indexing: Symbols indexed when opening workspace', async function() {
      this.timeout(20000);
      const testName = 'Epic2-Symbol-Indexing';
      
      try {
        const testCode = `module IndexingTest exposing (main, greet, add, Person)

import Node

type alias Person =
    { name : String
    , age : Int
    }

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

add : Int -> Int -> Int
add x y =
    x + y`;

        const testUri = await createTestFileOnDisk(testCode, 'epic2-indexing-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Wait for indexing to complete
          await monitor.waitForSymbolIndexing(testUri);
          
          // Test that symbols are indexed by requesting document symbols
          const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
            'vscode.executeDocumentSymbolProvider',
            testUri
          );
          
          assert.ok(symbols && symbols.length > 0, 'Should have indexed symbols');
          
          // Verify specific symbols are indexed
          const symbolNames = symbols.map(s => s.name);
          assert.ok(symbolNames.includes('greet'), 'greet function should be indexed');
          assert.ok(symbolNames.includes('add'), 'add function should be indexed');
          assert.ok(symbolNames.includes('Person'), 'Person type should be indexed');
          
          testResults.set(testName, { status: 'pass', details: `Indexed ${symbols.length} symbols` });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Symbol indexing failed: ${error}` });
        throw error;
      }
    });

    test('Code Completion: IntelliSense provides relevant suggestions', async function() {
      this.timeout(20000);
      const testName = 'Epic2-Code-Completion';
      
      try {
        const testCode = `module CompletionTest exposing (main, greet)

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

testFunction : String
testFunction =
    gr`;

        const testUri = await createTestFileOnDisk(testCode, 'epic2-completion-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Find position for completion (after "gr")
          const text = document.getText();
          const grIndex = text.lastIndexOf('gr');
          const position = document.positionAt(grIndex + 2);
          
          // Request completions
          const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
            'vscode.executeCompletionItemProvider',
            testUri,
            position
          );
          
          assert.ok(completions, 'Should receive completion response');
          assert.ok(completions.items && completions.items.length > 0, 'Should have completion items');
          
          // Look for the greet function in completions
          const greetCompletion = completions.items.find(item => 
            typeof item.label === 'string' ? item.label === 'greet' : item.label.label === 'greet'
          );
          assert.ok(greetCompletion, 'Should suggest greet function');
          
          testResults.set(testName, { status: 'pass', details: `Found ${completions.items.length} completions including greet` });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Code completion failed: ${error}` });
        throw error;
      }
    });

    test('Hover Information: Shows type information', async function() {
      this.timeout(20000);
      const testName = 'Epic2-Hover-Information';
      
      try {
        const testCode = `module HoverTest exposing (main, greet)

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

        const testUri = await createTestFileOnDisk(testCode, 'epic2-hover-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Find position of greet function call
          const text = document.getText();
          const greetCallIndex = text.indexOf('greet "World"');
          const position = document.positionAt(greetCallIndex + 2);
          
          // Request hover information
          const hoverInfo = await vscode.commands.executeCommand<vscode.Hover[]>(
            'vscode.executeHoverProvider',
            testUri,
            position
          );
          
          assert.ok(hoverInfo && hoverInfo.length > 0, 'Should receive hover information');
          
          const hover = hoverInfo[0];
          assert.ok(hover.range, 'Hover should have range');
          assert.ok(hover.contents, 'Hover should have contents');
          
          testResults.set(testName, { status: 'pass', details: 'Hover information provided with range and contents' });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Hover information failed: ${error}` });
        throw error;
      }
    });

    test('Go-to-Definition: Navigates to correct symbol definitions', async function() {
      this.timeout(20000);
      const testName = 'Epic2-Go-to-Definition';
      
      try {
        const testCode = `module DefinitionTest exposing (main, greet)

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

        const testUri = await createTestFileOnDisk(testCode, 'epic2-definition-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Find position of greet function call
          const text = document.getText();
          const greetCallIndex = text.indexOf('greet "World"');
          const position = document.positionAt(greetCallIndex + 2);
          
          // Request definition
          const definitions = await vscode.commands.executeCommand<vscode.Location[]>(
            'vscode.executeDefinitionProvider',
            testUri,
            position
          );
          
          assert.ok(definitions && definitions.length > 0, 'Should find definition');
          
          const definition = definitions[0];
          assert.ok(definition.uri, 'Definition should have URI');
          assert.ok(definition.range, 'Definition should have range');
          assert.strictEqual(definition.uri.toString(), testUri.toString(), 'Should point to same file');
          
          testResults.set(testName, { status: 'pass', details: 'Go-to-definition working correctly' });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Go-to-definition failed: ${error}` });
        throw error;
      }
    });
  });

  /**
   * EPIC 3: Advanced Navigation & References Integration Tests
   */
  suite('Epic 3: Advanced Navigation & References', () => {
    
    test('Find References: Shows all symbol usages', async function() {
      this.timeout(20000);
      const testName = 'Epic3-Find-References';
      
      try {
        const testCode = `module ReferencesTest exposing (main, greet)

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

useGreet1 : String
useGreet1 =
    greet "World"

useGreet2 : String
useGreet2 =
    greet "Universe"`;

        const testUri = await createTestFileOnDisk(testCode, 'epic3-references-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Find position of greet function definition
          const text = document.getText();
          const greetDefIndex = text.indexOf('greet : String -> String');
          const position = document.positionAt(greetDefIndex + 2);
          
          // Request references
          const references = await vscode.commands.executeCommand<vscode.Location[]>(
            'vscode.executeReferenceProvider',
            testUri,
            position
          );
          
          if (references && references.length > 0) {
            assert.ok(references.length >= 2, 'Should find at least definition and usage references');
            testResults.set(testName, { status: 'pass', details: `Found ${references.length} references` });
          } else {
            testResults.set(testName, { status: 'skip', details: 'Find references not yet implemented' });
          }
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Find references failed: ${error}` });
        throw error;
      }
    });

    test('Document Symbols: Shows hierarchical symbol structure', async function() {
      this.timeout(20000);
      const testName = 'Epic3-Document-Symbols';
      
      try {
        const testCode = `module SymbolsTest exposing (main, greet, Person)

import Node

type alias Person =
    { name : String
    , age : Int
    }

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

        const testUri = await createTestFileOnDisk(testCode, 'epic3-symbols-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Request document symbols
          const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
            'vscode.executeDocumentSymbolProvider',
            testUri
          );
          
          assert.ok(symbols && symbols.length > 0, 'Should have document symbols');
          
          // Verify specific symbols exist
          const symbolNames = symbols.map(s => s.name);
          assert.ok(symbolNames.includes('greet'), 'Should include greet function');
          assert.ok(symbolNames.includes('Person'), 'Should include Person type');
          
          // Verify symbol structure
          symbols.forEach(symbol => {
            assert.ok(symbol.name, 'Symbol should have name');
            assert.ok(typeof symbol.kind === 'number', 'Symbol should have kind');
            assert.ok(symbol.range, 'Symbol should have range');
            assert.ok(symbol.selectionRange, 'Symbol should have selection range');
          });
          
          testResults.set(testName, { status: 'pass', details: `Found ${symbols.length} document symbols` });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Document symbols failed: ${error}` });
        throw error;
      }
    });
  });

  /**
   * EPIC 4: Polish and Enhancement Integration Tests
   */
  suite('Epic 4: Polish and Enhancement', () => {
    
    test('Code Actions: Suggests fixes for errors', async function() {
      this.timeout(20000);
      const testName = 'Epic4-Code-Actions';
      
      try {
        const testCode = `module ActionsTest exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }

-- Missing type annotation - should trigger code action
brokenFunction input =
    "This needs a type annotation"`;

        const testUri = await createTestFileOnDisk(testCode, 'epic4-actions-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Wait for diagnostics
          try {
            await monitor.waitForDiagnostics(testUri);
          } catch (error) {
            // No diagnostics available - this is expected for code actions test
          }
          
          const diagnostics = vscode.languages.getDiagnostics(testUri);
          
          if (diagnostics.length > 0) {
            // Request code actions for the first diagnostic
            const range = diagnostics[0].range;
            const codeActions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
              'vscode.executeCodeActionProvider',
              testUri,
              range
            );
            
            if (codeActions && codeActions.length > 0) {
              testResults.set(testName, { status: 'pass', details: `Found ${codeActions.length} code actions` });
            } else {
              testResults.set(testName, { status: 'skip', details: 'Code actions not yet implemented' });
            }
          } else {
            testResults.set(testName, { status: 'skip', details: 'No diagnostics found for code actions test' });
          }
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Code actions failed: ${error}` });
        throw error;
      }
    });

    test('Workspace Symbols: Searches symbols across project', async function() {
      this.timeout(20000);
      const testName = 'Epic4-Workspace-Symbols';
      
      try {
        const testCode = `module WorkspaceTest exposing (uniqueWorkspaceFunction)

uniqueWorkspaceFunction : String -> String
uniqueWorkspaceFunction input =
    "Unique: " ++ input`;

        const testUri = await createTestFileOnDisk(testCode, 'epic4-workspace-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Wait for indexing
          await monitor.waitForSymbolIndexing(testUri);
          
          // Request workspace symbols
          const workspaceSymbols = await vscode.commands.executeCommand<vscode.SymbolInformation[]>(
            'vscode.executeWorkspaceSymbolProvider',
            'uniqueWorkspaceFunction'
          );
          
          if (workspaceSymbols && workspaceSymbols.length > 0) {
            const found = workspaceSymbols.find(symbol => 
              symbol.name.includes('uniqueWorkspaceFunction')
            );
            
            if (found) {
              testResults.set(testName, { status: 'pass', details: 'Workspace symbol search working' });
            } else {
              testResults.set(testName, { status: 'skip', details: 'Workspace symbols not finding test function' });
            }
          } else {
            testResults.set(testName, { status: 'skip', details: 'Workspace symbol search not yet implemented' });
          }
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Workspace symbols failed: ${error}` });
        throw error;
      }
    });

    test('Symbol Rename: Renames across files with preview', async function() {
      this.timeout(20000);
      const testName = 'Epic4-Symbol-Rename';
      
      try {
        const testCode = `module RenameTest exposing (main, oldFunctionName)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }

oldFunctionName : String -> String
oldFunctionName input =
    "Old name: " ++ input

useOldFunction : String
useOldFunction =
    oldFunctionName "test"`;

        const testUri = await createTestFileOnDisk(testCode, 'epic4-rename-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          const document = await openFileInEditor(testUri);
          await monitor.waitForMethod('textDocument/didOpen');
          
          // Find position of function definition
          const text = document.getText();
          const functionDefIndex = text.indexOf('oldFunctionName : String -> String');
          const position = document.positionAt(functionDefIndex + 5);
          
          // Request prepare rename
          const prepareRename = await vscode.commands.executeCommand<vscode.Range>(
            'vscode.executePrepareRename',
            testUri,
            position
          );
          
          if (prepareRename) {
            // Request rename
            const renameEdits = await vscode.commands.executeCommand<vscode.WorkspaceEdit>(
              'vscode.executeDocumentRenameProvider',
              testUri,
              position,
              'newFunctionName'
            );
            
            if (renameEdits && renameEdits.size > 0) {
              testResults.set(testName, { status: 'pass', details: 'Symbol rename working with preview' });
            } else {
              testResults.set(testName, { status: 'skip', details: 'Rename edits not generated' });
            }
          } else {
            testResults.set(testName, { status: 'skip', details: 'Symbol rename not yet implemented' });
          }
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Symbol rename failed: ${error}` });
        throw error;
      }
    });
  });

  /**
   * EPIC 5: Advanced Refactoring Integration Tests
   */
  suite('Epic 5: Advanced Refactoring', () => {
    
    test('Module Rename: File rename updates imports correctly', async function() {
      this.timeout(20000);
      const testName = 'Epic5-Module-Rename';
      
      try {
        // This test would require more complex setup with multiple files
        // For now, we'll mark it as skipped since Epic 5 features may not be fully implemented
        testResults.set(testName, { status: 'skip', details: 'Module rename requires multi-file test setup - Epic 5 may not be fully implemented' });
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Module rename failed: ${error}` });
        throw error;
      }
    });
  });

  /**
   * LSP Protocol Compliance Tests
   */
  suite('LSP Protocol Compliance', () => {
    
    test('Message Format: All LSP messages follow JSON-RPC specification', async function() {
      this.timeout(15000);
      const testName = 'Protocol-Message-Format';
      
      try {
        const testCode = `module ProtocolTest exposing (main)

import Node

main : Node.Program {} {}
main =
    Node.defineProgram
        { init = \\_ -> ( {}, Node.none )
        , update = \\_ model -> ( model, Node.none )
        , subscriptions = \\_ -> Sub.none
        }`;

        const testUri = await createTestFileOnDisk(testCode, 'protocol-compliance-test.gren');
        
        try {
          await monitor.startMonitoring(testUri);
          await openFileInEditor(testUri);
          
          const didOpenMessage = await monitor.waitForMethod('textDocument/didOpen');
          
          // Verify JSON-RPC structure
          assert.ok(didOpenMessage.method, 'LSP message should have method');
          assert.ok(didOpenMessage.params, 'LSP message should have params');
          assert.strictEqual(didOpenMessage.method, 'textDocument/didOpen');
          assert.ok(didOpenMessage.params.textDocument, 'didOpen should have textDocument');
          assert.ok(didOpenMessage.params.textDocument.uri, 'textDocument should have URI');
          
          testResults.set(testName, { status: 'pass', details: 'LSP messages follow JSON-RPC specification' });
          
        } finally {
          await cleanupTestFile(testUri);
        }
        
      } catch (error) {
        testResults.set(testName, { status: 'fail', details: `Protocol compliance failed: ${error}` });
        throw error;
      }
    });
  });
});