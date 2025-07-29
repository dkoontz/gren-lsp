import * as path from "path";
import * as fs from "fs";
import { workspace, ExtensionContext, window } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
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
    
    console.log('Extension path:', context.extensionPath);
    console.log('Searching for server binary in:', possiblePaths);
    
    for (const candidatePath of possiblePaths) {
      if (fs.existsSync(candidatePath)) {
        console.log('Found server binary at:', candidatePath);
        serverPath = candidatePath;
        break;
      }
    }
    
    if (!serverPath) {
      serverPath = possiblePaths[0]; // Fallback to first option
      console.log('No server binary found, using fallback:', serverPath);
    }
  }

  console.log('Using server path:', serverPath);
  
  // Check if server binary exists
  if (!fs.existsSync(serverPath)) {
    const errorMsg = `LSP server binary not found at: ${serverPath}`;
    console.error(errorMsg);
    window.showErrorMessage(errorMsg);
    return;
  }
  
  // Check if server binary is executable
  try {
    fs.accessSync(serverPath, fs.constants.F_OK | fs.constants.X_OK);
  } catch (err) {
    const errorMsg = `LSP server binary is not executable: ${serverPath}`;
    console.error(errorMsg, err);
    window.showErrorMessage(errorMsg);
    return;
  }
  
  const serverExecutable: Executable = {
    command: serverPath,
    args: [],
    options: {
      env: {
        ...process.env,
        RUST_LOG: config.get<string>('trace.server') === 'verbose' ? 'gren_lsp=debug' : 'gren_lsp=info'
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
    outputChannelName: "Gren LSP",
  };

  // Create the language client and start the client.
  client = new LanguageClient(
    "gren-lsp",
    "Gren Language Server",
    serverOptions,
    clientOptions
  );

  // Show log file location to user
  const tempDir = require('os').tmpdir();
  const logPath = path.join(tempDir, 'gren-lsp', 'server.log');
  console.log(`LSP server logs will be written to: ${logPath}`);
  
  // Start the client. This will also launch the server
  client.start().then(() => {
    console.log('Gren LSP client started successfully');
    console.log(`Check server logs at: ${logPath}`);
  }).catch(err => {
    console.error('Failed to start Gren LSP server:', err);
    window.showErrorMessage(`Failed to start Gren LSP server: ${err.message}\n\nCheck logs at: ${logPath}`);
  });
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
