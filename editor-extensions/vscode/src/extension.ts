import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { workspace, ExtensionContext, window, OutputChannel } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
  State,
} from "vscode-languageclient/node";

let client: LanguageClient;
let outputChannel: OutputChannel;

export function activate(context: ExtensionContext) {
  // Create output channel for extension logs (LSP client will create server channel automatically)
  outputChannel = window.createOutputChannel("Gren LSP Extension");
  
  outputChannel.appendLine("Gren LSP Extension starting...");
  outputChannel.appendLine("📺 Created Extension output channel (LSP client will create Server channel)");
  
  // Get the LSP server path from configuration or use default
  const config = workspace.getConfiguration('grenLsp');
  let serverPath = config.get<string>('serverPath', '');
  
  if (!serverPath) {
    // Try multiple possible locations for the server binary
    const possiblePaths = [
      // From extension directory to workspace root
      path.join(context.extensionPath, '..', '..', 'target', 'debug', 'gren-lsp'),
      // From extension directory to workspace root (alternative structure)
      path.join(context.extensionPath, '..', '..', '..', 'target', 'debug', 'gren-lsp'),
      // Absolute path if we're in the dev environment
      '/Users/david/dev/gren-lsp/target/debug/gren-lsp',
      // Try to find in current workspace folders
      ...(workspace.workspaceFolders || []).map(folder => 
        path.join(folder.uri.fsPath, 'target', 'debug', 'gren-lsp')
      )
    ];
    
    outputChannel.appendLine(`Extension path: ${context.extensionPath}`);
    outputChannel.appendLine(`Searching for server binary in: ${possiblePaths.join(', ')}`);
    
    for (const candidatePath of possiblePaths) {
      if (fs.existsSync(candidatePath)) {
        outputChannel.appendLine(`✅ Found server binary at: ${candidatePath}`);
        serverPath = candidatePath;
        break;
      } else {
        outputChannel.appendLine(`❌ Not found: ${candidatePath}`);
      }
    }
    
    if (!serverPath) {
      serverPath = possiblePaths[0]; // Fallback to first option
      outputChannel.appendLine(`⚠️ No server binary found, using fallback: ${serverPath}`);
    }
  }

  outputChannel.appendLine(`Using server path: ${serverPath}`);
  
  // Check if server binary exists
  if (!fs.existsSync(serverPath)) {
    const errorMsg = `❌ LSP server binary not found at: ${serverPath}`;
    outputChannel.appendLine(errorMsg);
    outputChannel.show(true);
    window.showErrorMessage(errorMsg);
    return;
  }
  
  // Check if server binary is executable
  try {
    fs.accessSync(serverPath, fs.constants.F_OK | fs.constants.X_OK);
    outputChannel.appendLine(`✅ Server binary is accessible and executable`);
  } catch (err) {
    const errorMsg = `❌ LSP server binary is not executable: ${serverPath}`;
    outputChannel.appendLine(`${errorMsg} - Error: ${err}`);
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
      outputChannel.appendLine(`🌳 Parse tree export enabled, directory: ${parseTreeDir}`);
      
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
      outputChannel.appendLine(`${errorMsg} - Error: ${err}`);
      window.showErrorMessage(`${errorMsg}\n\nDisabling parse tree export.`);
      parseTreeArgs = []; // Disable if directory setup fails
    }
  }
  
  // Configure server executable
  const rustLogLevel = config.get<string>('trace.server') === 'verbose' ? 'gren_lsp=debug' : 'gren_lsp=info';
  outputChannel.appendLine(`🔧 Server args: ${parseTreeArgs.join(' ')}`);
  outputChannel.appendLine(`📊 RUST_LOG level: ${rustLogLevel}`);
  
  const serverExecutable: Executable = {
    command: serverPath,
    args: parseTreeArgs,
    options: {
      env: {
        ...process.env,
        RUST_LOG: rustLogLevel
      }
    }
  };

  const serverOptions: ServerOptions = serverExecutable;

  // Options to control the language client
  const clientOptions: LanguageClientOptions = {
    // Register the server for Gren documents
    documentSelector: [{ scheme: "file", language: "gren" }],
    synchronize: {
      // Notify the server about file changes to Gren files
      fileEvents: workspace.createFileSystemWatcher("**/*.gren"),
    },
    outputChannelName: "Gren LSP Server",
    // Enable trusted markdown for clickable links in hover content
    markdown: {
      isTrusted: true,
    },
  };
  
  outputChannel.appendLine(`📋 Client options configured:`);
  outputChannel.appendLine(`  - Document selector: file:gren`);
  outputChannel.appendLine(`  - File watcher: **/*.gren`);
  outputChannel.appendLine(`  - Output channel: ${clientOptions.outputChannelName}`);

  // Create the language client and start the client.
  outputChannel.appendLine(`🚀 Creating LSP client...`);
  client = new LanguageClient(
    "gren-lsp",
    "Gren Language Server",
    serverOptions,
    clientOptions
  );
  
  // Add state change monitoring to track connection lifecycle
  client.onDidChangeState((stateChangeEvent) => {
    const oldState = stateChangeEvent.oldState === State.Stopped ? "Stopped" :
                     stateChangeEvent.oldState === State.Starting ? "Starting" : "Running";
    const newState = stateChangeEvent.newState === State.Stopped ? "Stopped" :
                     stateChangeEvent.newState === State.Starting ? "Starting" : "Running";
    
    outputChannel.appendLine(`🔄 LSP client state changed: ${oldState} → ${newState}`);
    
    if (newState === "Running") {
      outputChannel.appendLine(`✅ LSP client successfully connected to server!`);
      outputChannel.appendLine(`📺 "Gren LSP Server" output channel should now be visible`);
    }
  });

  // Show log file location to user
  const tempDir = require('os').tmpdir();
  const logPath = path.join(tempDir, 'gren-lsp', 'server.log');
  outputChannel.appendLine(`📄 LSP server logs will be written to: ${logPath}`);
  outputChannel.appendLine(`💡 To debug server issues, check this log file or run manually:`);
  outputChannel.appendLine(`   ${serverPath} --help`);
  
  // Start the client. This will also launch the server
  outputChannel.appendLine(`⚡ Starting LSP client...`);
  outputChannel.appendLine(`📋 Server command: ${serverPath} ${parseTreeArgs.join(' ')}`);
  outputChannel.appendLine(`🌍 Environment: RUST_LOG=${rustLogLevel}`);
  
  const startTime = Date.now();
  client.start().then(() => {
    const duration = Date.now() - startTime;
    outputChannel.appendLine(`✅ Gren LSP client started successfully (${duration}ms)`);
    outputChannel.appendLine(`📄 Check server logs at: ${logPath}`);
    outputChannel.appendLine(`🎉 Extension is now active and ready!`);
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
  }).catch(err => {
    const duration = Date.now() - startTime;
    outputChannel.appendLine(`❌ Failed to start Gren LSP server after ${duration}ms`);
    outputChannel.appendLine(`Error details: ${err.message}`);
    outputChannel.appendLine(`Stack trace: ${err.stack || 'No stack trace available'}`);
    outputChannel.show(true);
    window.showErrorMessage(`Failed to start Gren LSP server: ${err.message}\n\nCheck "Gren LSP Extension" output for details.`);
  });
}

export function deactivate(): Thenable<void> | undefined {
  if (outputChannel) {
    outputChannel.appendLine('🛑 Deactivating Gren LSP extension...');
  }
  
  if (!client) {
    if (outputChannel) {
      outputChannel.appendLine('⚠️ No client to stop');
      outputChannel.dispose();
    }
    return undefined;
  }
  
  if (outputChannel) {
    outputChannel.appendLine('🔌 Stopping LSP client...');
  }
  
  return client.stop().then(() => {
    if (outputChannel) {
      outputChannel.appendLine('✅ LSP client stopped successfully');
      outputChannel.dispose();
    }
  }).catch(err => {
    if (outputChannel) {
      outputChannel.appendLine(`❌ Error stopping LSP client: ${err.message}`);
      outputChannel.dispose();
    }
    throw err;
  });
}
