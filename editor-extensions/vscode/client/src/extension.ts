import * as path from "path";
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
    // Try to find the server in the workspace first, then in PATH
    const workspaceServerPath = path.join(context.extensionPath, '..', '..', 'target', 'debug', 'gren-lsp');
    serverPath = workspaceServerPath;
  }

  const serverExecutable: Executable = {
    command: serverPath,
    args: [],
    options: {
      env: {
        ...process.env,
        RUST_LOG: config.get<string>('trace.server') === 'verbose' ? 'debug' : 'info'
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

  // Start the client. This will also launch the server
  client.start().catch(err => {
    window.showErrorMessage(`Failed to start Gren LSP server: ${err.message}`);
  });
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
