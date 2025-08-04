import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { workspace, ExtensionContext, window, OutputChannel, commands } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
  State,
  Trace,
} from "vscode-languageclient/node";

import { GrenCompilerManager } from "./compiler-manager";
import { getLogger, closeLogger, FileLogger } from "./file-logger";

let client: LanguageClient;
let outputChannel: OutputChannel;
let compilerManager: GrenCompilerManager;
let fileLogger: FileLogger;

// Helper function to log to both output channel and file
function logToAll(message: string): void {
  outputChannel.appendLine(message);
  fileLogger.log('Extension', message);
}

export function activate(context: ExtensionContext) {
  console.log("Gren LSP Extension: Starting activation...");
  
  // Initialize file logger first (clears previous logs)
  fileLogger = getLogger();
  
  // Create output channel for extension logs (LSP client will create server channel automatically)
  outputChannel = window.createOutputChannel("Gren LSP Extension");
  
  logToAll("Gren LSP Extension starting...");
  logToAll("📺 Created Extension output channel (LSP client will create Server channel)");
  logToAll(`📄 Debug log file: ${fileLogger.getLogFilePath()}`);
  outputChannel.show(true); // Make sure the output channel is visible
  
  console.log("Gren LSP Extension: Output channel created");
  
  // Initialize compiler manager
  compilerManager = new GrenCompilerManager(context, outputChannel);
  
  console.log("Gren LSP Extension: Compiler manager created");
  
  // Register commands
  context.subscriptions.push(
    commands.registerCommand('grenLsp.downloadCompiler', () => compilerManager.downloadCompilerCommand()),
    commands.registerCommand('grenLsp.showCompilerVersion', () => compilerManager.showCompilerVersionCommand()),
    commands.registerCommand('grenLsp.testServerConnection', () => {
      const timestamp = new Date().toISOString().substring(11, 23);
      logToAll(`🔍 [${timestamp}] Testing server connection...`);
      logToAll(`   Client state: ${client.state === State.Stopped ? 'Stopped' : client.state === State.Starting ? 'Starting' : 'Running'}`);
      
      // Try to send a simple request to test if server responds
      if (client.state === State.Running) {
        client.sendRequest('workspace/diagnostic/refresh').then(() => {
          logToAll(`✅ [${new Date().toISOString().substring(11, 23)}] Server responded to test request!`);
        }).catch((error) => {
          logToAll(`❌ [${new Date().toISOString().substring(11, 23)}] Server failed to respond: ${error}`);
        });
      } else {
        logToAll(`❌ Client is not running - cannot test server connection`);
      }
    })
  );
  
  // Get the LSP server path from configuration or use default
  const config = workspace.getConfiguration('grenLsp');
  let serverPath = config.get<string>('serverPath', '');
  
  if (!serverPath) {
    // Try multiple possible locations for the server binary
    const possiblePaths = [
      // From extension directory to workspace root (correct lsp-server subdirectory)
      path.join(context.extensionPath, '..', '..', 'lsp-server', 'target', 'debug', 'gren-lsp'),
      // From extension directory to workspace root (alternative structure)
      path.join(context.extensionPath, '..', '..', '..', 'lsp-server', 'target', 'debug', 'gren-lsp'),
      // Absolute path if we're in the dev environment (correct location)
      '/Users/david/dev/gren-lsp/lsp-server/target/debug/gren-lsp',
      // Try to find in current workspace folders (correct lsp-server subdirectory)
      ...(workspace.workspaceFolders || []).map(folder => 
        path.join(folder.uri.fsPath, 'lsp-server', 'target', 'debug', 'gren-lsp')
      ),
      // Legacy paths (keep for backwards compatibility)
      path.join(context.extensionPath, '..', '..', 'target', 'debug', 'gren-lsp'),
      path.join(context.extensionPath, '..', '..', '..', 'target', 'debug', 'gren-lsp'),
      '/Users/david/dev/gren-lsp/target/debug/gren-lsp',
      ...(workspace.workspaceFolders || []).map(folder => 
        path.join(folder.uri.fsPath, 'target', 'debug', 'gren-lsp')
      )
    ];
    
    logToAll(`Extension path: ${context.extensionPath}`);
    logToAll(`Searching for server binary in: ${possiblePaths.join(', ')}`);
    
    for (const candidatePath of possiblePaths) {
      if (fs.existsSync(candidatePath)) {
        logToAll(`✅ Found server binary at: ${candidatePath}`);
        serverPath = candidatePath;
        break;
      } else {
        logToAll(`❌ Not found: ${candidatePath}`);
      }
    }
    
    if (!serverPath) {
      serverPath = possiblePaths[0]; // Fallback to first option
      logToAll(`⚠️ No server binary found, using fallback: ${serverPath}`);
    }
  }

  logToAll(`Using server path: ${serverPath}`);
  
  // Check if server binary exists
  if (!fs.existsSync(serverPath)) {
    const errorMsg = `❌ LSP server binary not found at: ${serverPath}`;
    logToAll(errorMsg);
    outputChannel.show(true);
    window.showErrorMessage(errorMsg);
    return;
  }
  
  // Check if server binary is executable
  try {
    fs.accessSync(serverPath, fs.constants.F_OK | fs.constants.X_OK);
    logToAll(`✅ Server binary is accessible and executable`);
  } catch (err) {
    const errorMsg = `❌ LSP server binary is not executable: ${serverPath}`;
    logToAll(`${errorMsg} - Error: ${err}`);
    outputChannel.show(true);
    window.showErrorMessage(errorMsg);
    return;
  }

  // Handle debug parse tree export settings
  const exportParseTree = config.get<boolean>('debug.exportParseTree', false);
  let parseTreeArgs: string[] = [];
  
  if (exportParseTree) {
    let parseTreeDir = config.get<string>('debug.parseTreeDirectory', '');
    
    if (!parseTreeDir) {
      // Use default directory in temp folder
      parseTreeDir = path.join(os.tmpdir(), 'gren-lsp-parse-trees');
    }
    
    // Ensure directory exists
    try {
      if (!fs.existsSync(parseTreeDir)) {
        fs.mkdirSync(parseTreeDir, { recursive: true });
      }
      
      // Verify we can write to the directory
      fs.accessSync(parseTreeDir, fs.constants.W_OK);
      
      parseTreeArgs = ['--debug-export-trees', parseTreeDir];
      logToAll(`🌳 Parse tree export enabled, directory: ${parseTreeDir}`);
      
      // Show user notification about debug mode
      window.showInformationMessage(
        `Gren LSP: Parse tree debug export is enabled. Trees will be saved to: ${parseTreeDir}`,
        'Open Folder'
      ).then(selection => {
        if (selection === 'Open Folder') {
          // Open the folder in file explorer/finder (cross-platform)
          const { exec } = require('child_process');
          const command = process.platform === 'win32' ? 'explorer' : 
                         process.platform === 'darwin' ? 'open' : 'xdg-open';
          exec(`${command} "${parseTreeDir}"`);
        }
      });
      
    } catch (err) {
      const errorMsg = `❌ Failed to create or access parse tree directory: ${parseTreeDir}`;
      logToAll(`${errorMsg} - Error: ${err}`);
      window.showErrorMessage(`${errorMsg}\\n\\nDisabling parse tree export.`);
      parseTreeArgs = []; // Disable if directory setup fails
    }
  }
  
  // Initialize and start LSP server after resolving compiler
  logToAll(`🔍 Resolving Gren compiler before starting LSP server...`);
  console.log("Gren LSP Extension: Starting compiler resolution...");
  
  // Wait for compiler resolution before starting server
  compilerManager.getCompilerPath().then((grenCompilerPath) => {
    console.log("Gren LSP Extension: Compiler resolution completed:", grenCompilerPath);
    
    if (!grenCompilerPath) {
      const errorMsg = `❌ No Gren compiler found. LSP server cannot start without a compiler.`;
      logToAll(errorMsg);
      outputChannel.show(true);
      console.error("Gren LSP Extension: No compiler found");
      window.showErrorMessage(`${errorMsg}\n\nPlease install Gren compiler or use the extension's download feature.`);
      return;
    }

    logToAll(`✅ Gren compiler found: ${grenCompilerPath}`);
    logToAll(`🚀 Starting LSP server with compiler path...`);
    console.log("Gren LSP Extension: Starting LSP server with compiler:", grenCompilerPath);

    // Configure server executable with the resolved compiler path
    const traceLevel = config.get<string>('trace.server', 'off');
    const rustLogLevel = traceLevel === 'verbose' ? 'gren_lsp=debug' : 'gren_lsp=info';
    logToAll(`🔧 Server args: ${parseTreeArgs.join(' ')}`);
    logToAll(`📊 RUST_LOG level: ${rustLogLevel}`);
    logToAll(`🛠️ GREN_COMPILER_PATH: ${grenCompilerPath}`);
    
    // Create server options with stderr capture
    const serverOptions: ServerOptions = () => {
      return new Promise((resolve, reject) => {
        const { spawn } = require('child_process');
        
        logToAll(`🚀 Spawning LSP server process: ${serverPath} ${parseTreeArgs.join(' ')}`);
        
        const serverProcess = spawn(serverPath, parseTreeArgs, {
          env: {
            ...process.env,
            RUST_LOG: rustLogLevel,
            GREN_COMPILER_PATH: grenCompilerPath
          },
          stdio: ['pipe', 'pipe', 'pipe'] // stdin, stdout, stderr
        });
        
        // Capture stderr and redirect to file logger
        serverProcess.stderr.on('data', (data: Buffer) => {
          fileLogger.logServerOutput(data.toString());
        });
        
        // Log process startup
        serverProcess.on('spawn', () => {
          logToAll(`✅ LSP server process spawned successfully (PID: ${serverProcess.pid})`);
        });
        
        // Handle process errors
        serverProcess.on('error', (error: Error) => {
          logToAll(`❌ LSP server process error: ${error.message}`);
          reject(error);
        });
        
        // Handle process exit
        serverProcess.on('exit', (code: number | null, signal: string | null) => {
          logToAll(`🔚 LSP server process exited (code: ${code}, signal: ${signal})`);
        });
        
        // Return the connection to the LSP client
        resolve({
          reader: serverProcess.stdout,
          writer: serverProcess.stdin
        });
      });
    };

    // Options to control the language client
    const clientOptions: LanguageClientOptions = {
      // Register the server for Gren documents
      documentSelector: [{ scheme: "file", language: "gren" }],
      synchronize: {
        // Notify the server about file changes to Gren files
        fileEvents: workspace.createFileSystemWatcher("**/*.gren"),
        // Synchronize the configuration section to the server
        configurationSection: 'grenLsp'
      },
      outputChannelName: "Gren LSP Server",
      // Enable trusted markdown for clickable links in hover content
      markdown: {
        isTrusted: true,
      }
    };
    
    logToAll(`📋 Client options configured:`);
    logToAll(`  - Document selector: file:gren`);
    logToAll(`  - File watcher: **/*.gren`);
    logToAll(`  - Output channel: ${clientOptions.outputChannelName}`);

    // Create the language client and start the client.
    logToAll(`🚀 Creating LSP client...`);
    client = new LanguageClient(
      "gren-lsp",
      "Gren Language Server",
      serverOptions,
      clientOptions
    );

    // Enable LSP protocol tracing if verbose mode is on
    if (traceLevel === 'verbose') {
      logToAll(`🔍 Enabling verbose LSP protocol tracing...`);
      client.setTrace(Trace.Verbose);
    }
    
    // Add state change monitoring to track connection lifecycle
    client.onDidChangeState((stateChangeEvent) => {
      const oldState = stateChangeEvent.oldState === State.Stopped ? "Stopped" :
                       stateChangeEvent.oldState === State.Starting ? "Starting" : "Running";
      const newState = stateChangeEvent.newState === State.Stopped ? "Stopped" :
                       stateChangeEvent.newState === State.Starting ? "Starting" : "Running";
      
      logToAll(`🔄 LSP client state changed: ${oldState} → ${newState}`);
      
      if (newState === "Running") {
        logToAll(`✅ LSP client successfully connected to server!`);
        logToAll(`📺 "Gren LSP Server" output channel should now be visible`);
      } else if (newState === "Stopped") {
        logToAll(`❌ LSP client stopped! Connection lost.`);
        outputChannel.show(true);
        window.showErrorMessage('Gren LSP server connection lost. Check output for errors.');
      }
    });

    // Show log file location to user
    const tempDir = require('os').tmpdir();
    const logPath = path.join(tempDir, 'gren-lsp', 'server.log');
    outputChannel.appendLine(`📄 LSP server logs will be written to: ${logPath}`);
    outputChannel.appendLine(`💡 To debug server issues, check this log file or run manually:`);
    outputChannel.appendLine(`   ${serverPath} --help`);
    
    // Start the client. This will also launch the server
    logToAll(`⚡ Starting LSP client...`);
    logToAll(`📋 Server command: ${serverPath} ${parseTreeArgs.join(' ')}`);
    logToAll(`🌍 Environment: RUST_LOG=${rustLogLevel}, GREN_COMPILER_PATH=${grenCompilerPath}`);
    console.log("Gren LSP Extension: About to start LSP client");
    
    const startTime = Date.now();
    client.start().then(() => {
      const duration = Date.now() - startTime;
      logToAll(`✅ Gren LSP client started successfully (${duration}ms)`);
      logToAll(`📄 LSP server logs are being captured to: ${fileLogger.getLogFilePath()}`);
      logToAll(`🎉 Extension is now active and ready!`);
      console.log("Gren LSP Extension: LSP client started successfully in", duration, "ms");
      outputChannel.appendLine(`\n💡 You should now see two channels in the Output panel:`);
      outputChannel.appendLine(`  - "Gren LSP Extension" (this channel) - Extension logs`);
      outputChannel.appendLine(`  - "Gren LSP Server" - LSP communication logs`);
      outputChannel.appendLine(`\n🔍 If you don't see "Gren LSP Server" channel:`);
      outputChannel.appendLine(`  1. The server may have crashed during startup`);
      outputChannel.appendLine(`  2. Check server logs at: ${logPath}`);
      outputChannel.appendLine(`  3. Check Developer Tools console for errors`);
      outputChannel.appendLine(`  4. Try running manually: ${serverPath} --help`);
      outputChannel.appendLine(`\n📋 Server process details:`);
      outputChannel.appendLine(`  - Command: ${serverPath}`);
      outputChannel.appendLine(`  - Args: ${parseTreeArgs.join(' ')}`);
      outputChannel.appendLine(`  - RUST_LOG: ${rustLogLevel}`);
      outputChannel.appendLine(`  - GREN_COMPILER_PATH: ${grenCompilerPath}`);
    }).catch(err => {
      const duration = Date.now() - startTime;
      const errorMsg = `❌ Failed to start Gren LSP server after ${duration}ms`;
      outputChannel.appendLine(errorMsg);
      outputChannel.appendLine(`Error details: ${err.message}`);
      outputChannel.appendLine(`Stack trace: ${err.stack || 'No stack trace available'}`);
      outputChannel.show(true);
      console.error("Gren LSP Extension: Failed to start LSP client:", err);
      window.showErrorMessage(`Failed to start Gren LSP server: ${err.message}\n\nCheck "Gren LSP Extension" output for details.`);
    });

  }).catch((error) => {
    const errorMsg = `⚠️ Error resolving compiler path: ${error}`;
    outputChannel.appendLine(errorMsg);
    outputChannel.show(true);
    console.error("Gren LSP Extension: Error resolving compiler path:", error);
    window.showErrorMessage(`${errorMsg}\n\nLSP server cannot start without a compiler.`);
  });
}

// Export the client for test access
export function getLanguageClient(): LanguageClient | undefined {
  return client;
}

export function deactivate(): Thenable<void> | undefined {
  if (outputChannel && fileLogger) {
    logToAll('🛑 Deactivating Gren LSP extension...');
  }
  
  if (!client) {
    if (outputChannel && fileLogger) {
      logToAll('⚠️ No client to stop');
      outputChannel.dispose();
      closeLogger();
    }
    return undefined;
  }
  
  if (outputChannel && fileLogger) {
    logToAll('🔌 Stopping LSP client...');
  }
  
  return client.stop().then(() => {
    if (outputChannel && fileLogger) {
      logToAll('✅ LSP client stopped successfully');
      outputChannel.dispose();
      closeLogger();
    }
  }).catch(err => {
    if (outputChannel && fileLogger) {
      logToAll(`❌ Error stopping LSP client: ${err.message}`);
      outputChannel.dispose();
      closeLogger();
    }
    throw err;
  });
}