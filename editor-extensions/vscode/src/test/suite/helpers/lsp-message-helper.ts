import * as vscode from "vscode";
import { testLogger, LogLevel } from "./test-logger";
import { LanguageClient, State } from "vscode-languageclient/node";
import { getLanguageClient } from "../../../extension";

// Global timeout constants for LSP operations
export const LSP_RESPONSE_TIMEOUT = 1000; // 1 second for LSP responses
export const LSP_EXTENSION_READY_TIMEOUT = 10000; // 10 seconds for extension activation

export interface LSPMessage {
  method: string;
  params?: any;
  id?: number | string;
  result?: any;
  error?: any;
  timestamp: number;
}

export interface LSPRequest extends LSPMessage {
  id: number | string;
  method: string;
  params: any;
}

export interface LSPResponse extends LSPMessage {
  id: number | string;
  result?: any;
  error?: any;
}

export interface LSPNotification extends LSPMessage {
  method: string;
  params: any;
}

export interface ObservedLSPMessage extends LSPMessage {
  direction: "outgoing" | "incoming";
  messageId?: string | number;
}

/**
 * ObservedLSPMessageMonitor intercepts actual LSP messages sent between VS Code and the LSP server
 * Uses pass-through interception to monitor real protocol communication without affecting behavior
 */
export class ObservedLSPMessageMonitor {
  private messages: ObservedLSPMessage[] = [];
  private client: LanguageClient | null = null;
  private originalSendNotification: any;
  private originalSendRequest: any;
  private isAttached: boolean = false;

  constructor() {
    testLogger.verbose("🔍 LSP message monitor created");
  }

  /**
   * Attach to the actual LanguageClient to intercept real LSP messages
   */
  async attachToClient(): Promise<void> {
    // Wait for extension to be ready and get the actual client
    await this.waitForExtensionReady();

    const clientOrUndefined = getLanguageClient();
    if (!clientOrUndefined) {
      throw new Error("❌ Could not get LanguageClient from extension");
    }
    this.client = clientOrUndefined;

    if (this.client.state !== State.Running) {
      throw new Error(
        `❌ LanguageClient is not running (state: ${this.client.state})`,
      );
    }

    testLogger.verbose(`✅ Got LanguageClient in state: ${this.client.state}`);

    // Store original methods
    this.originalSendNotification = this.client.sendNotification.bind(
      this.client,
    );
    this.originalSendRequest = this.client.sendRequest.bind(this.client);

    // Intercept outgoing notifications using a wrapper approach to preserve signatures
    const clientRef = this.client;
    const originalSendNotificationRef = this.originalSendNotification;
    const monitorRef = this;

    // Replace sendNotification with interceptor that preserves all overloaded signatures
    (this.client as any).sendNotification = function (...args: any[]) {
      // Extract method name from first argument (could be string or NotificationType)
      const methodName =
        typeof args[0] === "string"
          ? args[0]
          : args[0]?.method || args[0]?.name || "unknown";
      const params = args[1];

      monitorRef.recordMessage({
        method: methodName,
        params: params,
        direction: "outgoing",
        timestamp: Date.now(),
      });

      // Pass through with all original arguments
      return originalSendNotificationRef.apply(clientRef, args);
    };

    // Replace sendRequest with interceptor that preserves all overloaded signatures
    const originalSendRequestRef = this.originalSendRequest;
    (this.client as any).sendRequest = function (...args: any[]) {
      // Extract method name from first argument (could be string or RequestType)
      const methodName =
        typeof args[0] === "string"
          ? args[0]
          : args[0]?.method || args[0]?.name || "unknown";
      const params = args[1];

      const messageId = monitorRef.generateMessageId();
      monitorRef.recordMessage({
        method: methodName,
        params: params,
        direction: "outgoing",
        messageId: messageId,
        timestamp: Date.now(),
      });

      // Pass through with all original arguments and track response
      const promise = originalSendRequestRef.apply(clientRef, args);

      promise
        .then((result: any) => {
          monitorRef.recordMessage({
            method: methodName,
            result: result,
            direction: "incoming",
            messageId: messageId,
            timestamp: Date.now(),
          });
        })
        .catch((error: any) => {
          monitorRef.recordMessage({
            method: methodName,
            error: error,
            direction: "incoming",
            messageId: messageId,
            timestamp: Date.now(),
          });
        });

      return promise;
    };

    this.isAttached = true;
    testLogger.verbose("✅ LSP message monitor attached to LanguageClient");
  }

  /**
   * Start monitoring for a specific URI (compatibility with existing interface)
   */
  async startMonitoring(uri: vscode.Uri): Promise<void> {
    this.messages = [];
    testLogger.verbose(`🔍 Started LSP monitoring for ${uri.toString()}`);

    // With pre-activation, we should already be attached to the LanguageClient
    if (!this.isAttached) {
      throw new Error(
        "❌ LSP message monitor not attached - ensure preActivateExtension() was called",
      );
    }
  }

  /**
   * Pre-activate the extension by opening a dummy Gren file
   * This ensures the extension is active and LanguageClient is running before tests
   */
  async preActivateExtension(): Promise<void> {
    testLogger.verbose("🚀 Pre-activating Gren LSP extension...");

    // Create a dummy Gren file to trigger extension activation
    const dummyContent = `module ActivationTrigger exposing (..)

-- This file is used to trigger Gren LSP extension activation for testing
dummy : String
dummy = "activation trigger"`;

    const dummyUri = await createTestFileOnDisk(
      dummyContent,
      "ActivationTrigger.gren",
    );

    try {
      // Open the dummy file to trigger extension activation
      testLogger.verbose(
        "📁 Opening dummy file to trigger extension activation...",
      );
      await openFileInEditor(dummyUri);

      // Wait for extension to fully activate and LanguageClient to start
      testLogger.verbose(
        "⏳ Waiting for extension activation and LanguageClient startup...",
      );
      await this.waitForExtensionReady();

      testLogger.verbose("✅ Extension pre-activation completed successfully");
    } finally {
      // Clean up dummy file and close editors
      testLogger.verbose("🧹 Cleaning up activation trigger file...");
      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
      await cleanupTestFile(dummyUri);
    }
  }

  /**
   * Record an LSP message
   */
  private recordMessage(message: ObservedLSPMessage): void {
    this.messages.push(message);
    testLogger.verbose(
      `  LSP message: ${message.method} (${message.direction})`,
    );
  }

  /**
   * Generate a unique message ID for request/response correlation
   */
  private generateMessageId(): string {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Wait for extension to be ready (compatibility method)
   */
  private async waitForExtensionReady(
    timeoutMs: number = LSP_EXTENSION_READY_TIMEOUT,
  ): Promise<void> {
    const startTime = Date.now();
    testLogger.verbose(
      "⏳ Waiting for Gren LSP extension and client to be ready...",
    );

    while (Date.now() - startTime < timeoutMs) {
      const extension = vscode.extensions.getExtension("gren-lsp.gren-lsp");
      testLogger.verbose(
        `🔍 Extension found: ${!!extension}, isActive: ${extension?.isActive}`,
      );

      if (extension && extension.isActive) {
        const client = getLanguageClient();
        testLogger.verbose(
          `🔍 Client found: ${!!client}, state: ${client?.state}`,
        );

        if (client && client.state === State.Running) {
          testLogger.verbose("✅ Extension and client are ready!");
          return;
        } else if (client) {
          testLogger.verbose(`⏳ Client state is ${client.state}, waiting...`);
        }
      } else if (extension) {
        testLogger.verbose("⏳ Extension found but not active, waiting...");
      } else {
        testLogger.verbose("❌ Extension not found");
      }

      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    // Provide detailed error information
    const extension = vscode.extensions.getExtension("gren-lsp.gren-lsp");
    const client = getLanguageClient();
    throw new Error(
      `Timeout waiting for Gren LSP extension and client to be ready after ${timeoutMs}ms. ` +
        `Extension: ${extension ? "found" : "not found"}, ` +
        `Active: ${extension?.isActive}, ` +
        `Client: ${client ? "found" : "not found"}, ` +
        `State: ${client?.state || "N/A"}`,
    );
  }

  /**
   * Get all captured messages
   */
  getAllMessages(): ObservedLSPMessage[] {
    return [...this.messages];
  }

  /**
   * Get messages of a specific method
   */
  getMessagesForMethod(method: string): ObservedLSPMessage[] {
    return this.messages.filter((msg) => msg.method === method);
  }

  /**
   * Get the last message for a specific method
   */
  getLastMessageForMethod(method: string): ObservedLSPMessage | undefined {
    const messages = this.getMessagesForMethod(method);
    return messages[messages.length - 1];
  }

  /**
   * Wait for a specific LSP method to be called (compatibility method)
   */
  async waitForMethod(
    method: string,
    timeoutMs: number = LSP_RESPONSE_TIMEOUT,
  ): Promise<ObservedLSPMessage> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const message = this.getLastMessageForMethod(method);
      if (message) {
        return message;
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }

    throw new Error(
      `Timeout waiting for LSP method: ${method}. Captured methods: ${this.getMethodsList()}`,
    );
  }

  /**
   * Wait for diagnostics to be published by the LSP server
   */
  async waitForDiagnostics(
    uri: vscode.Uri,
    timeoutMs: number = 5000,
  ): Promise<vscode.Diagnostic[]> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const diagnostics = vscode.languages.getDiagnostics(uri);
      if (diagnostics.length > 0) {
        return diagnostics;
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    throw new Error(
      `Timeout waiting for diagnostics after ${timeoutMs}ms`,
    );
  }

  /**
   * Wait for diagnostics to be cleared by the LSP server
   */
  async waitForDiagnosticsCleared(
    uri: vscode.Uri,
    timeoutMs: number = 5000,
  ): Promise<void> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const diagnostics = vscode.languages.getDiagnostics(uri);
      if (diagnostics.length === 0) {
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    throw new Error(
      `Timeout waiting for diagnostics to be cleared after ${timeoutMs}ms`,
    );
  }

  /**
   * Wait for symbol indexing to complete after opening a document
   */
  async waitForSymbolIndexing(
    uri: vscode.Uri,
    timeoutMs: number = 3000,
  ): Promise<vscode.DocumentSymbol[]> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
        'vscode.executeDocumentSymbolProvider',
        uri
      );
      if (symbols && symbols.length > 0) {
        return symbols;
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    throw new Error(
      `Timeout waiting for symbol indexing after ${timeoutMs}ms`,
    );
  }

  /**
   * Get list of all captured method names
   */
  private getMethodsList(): string {
    const methods = [
      ...new Set(this.messages.map((msg) => `${msg.method}(${msg.direction})`)),
    ];
    return methods.join(", ");
  }

  /**
   * Clear all captured messages
   */
  clear(): void {
    this.messages = [];
  }

  /**
   * Stop monitoring and clean up
   */
  stopMonitoring(): void {
    testLogger.verbose(
      `🛑 Stopped LSP monitoring. Captured ${this.messages.length} messages`,
    );
  }

  /**
   * Kill the LSP server process for testing server failure scenarios
   */
  async killServerProcess(): Promise<void> {
    const client = getLanguageClient();
    if (!client) {
      throw new Error("❌ No LanguageClient available to kill server process");
    }

    const serverProcess = (client as any)._serverProcess;
    if (!serverProcess) {
      throw new Error("❌ No server process found in LanguageClient");
    }

    const pid = serverProcess.pid;
    if (!pid) {
      throw new Error("❌ Server process has no PID");
    }

    testLogger.verbose(`🔪 Killing LSP server process ${pid}...`);

    try {
      process.kill(pid, "SIGKILL");
      testLogger.verbose(`✅ Successfully killed LSP server process ${pid}`);
    } catch (error) {
      throw new Error(`❌ Failed to kill server process ${pid}: ${error}`);
    }
  }

  /**
   * Wait for LanguageClient to change to a specific state
   */
  async waitForClientStateChange(
    expectedState: State,
    timeoutMs: number = 10000,
  ): Promise<void> {
    const client = getLanguageClient();
    if (!client) {
      throw new Error("❌ No LanguageClient available to monitor state");
    }

    const startTime = Date.now();
    testLogger.verbose(
      `⏳ Waiting for client state change to ${this.getStateName(expectedState)}...`,
    );

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(
          new Error(
            `Timeout waiting for client state ${this.getStateName(expectedState)} after ${timeoutMs}ms. Current state: ${this.getStateName(client.state)}`,
          ),
        );
      }, timeoutMs);

      // Check if already in expected state
      if (client.state === expectedState) {
        clearTimeout(timeout);
        testLogger.verbose(
          `✅ Client already in state ${this.getStateName(expectedState)}`,
        );
        resolve();
        return;
      }

      const disposable = client.onDidChangeState((stateChange) => {
        testLogger.verbose(
          `🔄 Client state changed: ${this.getStateName(stateChange.oldState)} → ${this.getStateName(stateChange.newState)}`,
        );

        if (stateChange.newState === expectedState) {
          clearTimeout(timeout);
          disposable.dispose();
          testLogger.verbose(
            `✅ Client reached expected state ${this.getStateName(expectedState)}`,
          );
          resolve();
        }
      });
    });
  }

  /**
   * Reload the VS Code window to reset extension state
   */
  async reloadWindow(): Promise<void> {
    testLogger.verbose("🔄 Reloading VS Code window for extension reset...");
    testLogger.verbose("⚠️ Test execution will stop here due to window reload");

    // Execute the reload command - this will stop execution as the window reloads
    await vscode.commands.executeCommand("workbench.action.reloadWindow");
  }

  /**
   * Get human-readable state name
   */
  private getStateName(state: State): string {
    switch (state) {
      case State.Stopped:
        return "Stopped";
      case State.Starting:
        return "Starting";
      case State.Running:
        return "Running";
      default:
        return `Unknown(${state})`;
    }
  }

  /**
   * Dispose of all resources and restore original methods
   */
  dispose(): void {
    if (this.client && this.isAttached) {
      // Restore original methods to ensure no side effects
      if (this.originalSendNotification) {
        (this.client as any).sendNotification = this.originalSendNotification;
      }
      if (this.originalSendRequest) {
        (this.client as any).sendRequest = this.originalSendRequest;
      }
      testLogger.verbose("✅ Restored original LanguageClient methods");
    }

    this.isAttached = false;
    this.client = null;
    this.originalSendNotification = null;
    this.originalSendRequest = null;
  }
}

/**
 * Helper function to close all open editors and documents
 */
export async function closeAllFiles(): Promise<void> {
  await vscode.commands.executeCommand("workbench.action.closeAllEditors");

  // Wait a moment for editors to close
  await new Promise((resolve) => setTimeout(resolve, 100));

  testLogger.verbose(
    `🗃️ Closed all editors. Open documents: ${vscode.workspace.textDocuments.length}`,
  );
}

/**
 * Helper function to create a test file directly on disk using Node.js fs
 * This avoids loading the file into VS Code's workspace until we explicitly open it
 */
export async function createTestFileOnDisk(
  content: string,
  fileName: string = "test.gren",
): Promise<vscode.Uri> {
  const workspaceFolders = vscode.workspace.workspaceFolders;
  if (!workspaceFolders || workspaceFolders.length === 0) {
    throw new Error("No workspace folder available for creating test file");
  }

  const testUri = vscode.Uri.joinPath(workspaceFolders[0].uri, fileName);

  testLogger.verbose(
    `📁 Creating test file directly on disk: ${testUri.toString()}`,
  );

  // Use Node.js fs to write file directly to disk without loading into VS Code workspace
  const fs = require("fs");
  const path = require("path");

  // Ensure directory exists
  const dirPath = path.dirname(testUri.fsPath);
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }

  // Write file content directly to disk
  fs.writeFileSync(testUri.fsPath, content, "utf8");

  testLogger.verbose(`✅ File written directly to disk: ${fileName}`);

  return testUri;
}

/**
 * Helper function to open a file that exists on disk (triggers real textDocument/didOpen)
 */
export async function openFileInEditor(
  uri: vscode.Uri,
): Promise<vscode.TextDocument> {
  testLogger.verbose(`📂 Opening file in editor: ${uri.toString()}`);

  // This should trigger the real textDocument/didOpen LSP message
  const document = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(document);

  testLogger.verbose(
    `✅ File opened in editor. Language: ${document.languageId}`,
  );

  return document;
}

/**
 * Helper function to clean up test files
 */
export async function cleanupTestFile(uri: vscode.Uri): Promise<void> {
  try {
    const edit = new vscode.WorkspaceEdit();
    edit.deleteFile(uri);
    await vscode.workspace.applyEdit(edit);
  } catch (error) {
    console.log(
      `Note: Could not clean up test file ${uri.toString()}: ${error}`,
    );
  }
}
