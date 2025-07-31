import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { execSync } from 'child_process';

export interface GrenJson {
  'gren-version': string;
  type: 'application' | 'package';
  'source-directories': string[];
  dependencies: {
    direct: Record<string, string>;
    indirect: Record<string, string>;
  };
}

export class GrenCompilerManager {
  private context: vscode.ExtensionContext;
  private outputChannel: vscode.OutputChannel;

  constructor(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel) {
    this.context = context;
    this.outputChannel = outputChannel;
  }

  /**
   * Get the path to the Gren compiler, downloading if necessary
   */
  async getCompilerPath(): Promise<string | null> {
    const config = vscode.workspace.getConfiguration('grenLsp');
    
    // Check if user has specified a manual compiler path
    const manualPath = config.get<string>('compiler.path', '');
    if (manualPath) {
      if (fs.existsSync(manualPath)) {
        this.outputChannel.appendLine(`✅ Using manual compiler path: ${manualPath}`);
        return manualPath;
      } else {
        this.outputChannel.appendLine(`❌ Manual compiler path not found: ${manualPath}`);
        vscode.window.showErrorMessage(`Gren compiler not found at specified path: ${manualPath}`);
        return null;
      }
    }

    // Check if auto-download is enabled
    const autoDownload = config.get<boolean>('compiler.autoDownload', true);
    if (!autoDownload) {
      this.outputChannel.appendLine(`⚠️ Auto-download disabled, falling back to PATH lookup`);
      return this.findCompilerInPath();
    }

    // Try to find and download the compiler version from gren.json
    const requiredVersion = await this.getRequiredGrenVersion();
    if (!requiredVersion) {
      this.outputChannel.appendLine(`⚠️ No gren.json found, falling back to PATH lookup`);
      return this.findCompilerInPath();
    }

    this.outputChannel.appendLine(`🔍 Required Gren version: ${requiredVersion}`);

    // Check if we already have this version downloaded
    const cachedPath = await this.getCachedCompilerPath(requiredVersion);
    if (cachedPath) {
      this.outputChannel.appendLine(`✅ Using cached compiler: ${cachedPath}`);
      return cachedPath;
    }

    // Download the required version
    try {
      const downloadedPath = await this.downloadGrenCompiler(requiredVersion);
      this.outputChannel.appendLine(`✅ Downloaded compiler: ${downloadedPath}`);
      return downloadedPath;
    } catch (error) {
      this.outputChannel.appendLine(`❌ Failed to download compiler: ${error}`);
      vscode.window.showErrorMessage(`Failed to download Gren compiler v${requiredVersion}: ${error}. Auto-download has been temporarily disabled.`);
      
      // Temporarily disable auto-download to prevent repeated failures
      const config = vscode.workspace.getConfiguration('grenLsp');
      await config.update('compiler.autoDownload', false, vscode.ConfigurationTarget.Workspace);
      this.outputChannel.appendLine(`⚠️ Auto-download disabled for this workspace due to repeated failures`);
      
      // Fall back to PATH lookup
      this.outputChannel.appendLine(`⚠️ Falling back to PATH lookup`);
      return this.findCompilerInPath();
    }
  }

  /**
   * Find the required Gren version from gren.json in the workspace
   */
  private async getRequiredGrenVersion(): Promise<string | null> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
      return null;
    }

    // Check each workspace folder for gren.json
    for (const folder of workspaceFolders) {
      const grenJsonPath = path.join(folder.uri.fsPath, 'gren.json');
      
      if (fs.existsSync(grenJsonPath)) {
        try {
          const content = fs.readFileSync(grenJsonPath, 'utf8');
          const grenJson: GrenJson = JSON.parse(content);
          
          if (grenJson['gren-version']) {
            this.outputChannel.appendLine(`📋 Found gren.json in ${folder.name}: v${grenJson['gren-version']}`);
            return grenJson['gren-version'];
          }
        } catch (error) {
          this.outputChannel.appendLine(`❌ Failed to parse gren.json in ${folder.name}: ${error}`);
          continue;
        }
      }
    }

    return null;
  }

  /**
   * Check if we have a cached version of the compiler
   */
  private async getCachedCompilerPath(version: string): Promise<string | null> {
    const compilerDir = path.join(this.context.globalStorageUri.fsPath, 'gren-compilers', version);
    const compilerPath = path.join(compilerDir, 'bin', 'gren');
    
    if (fs.existsSync(compilerPath)) {
      // Verify the compiler works
      try {
        const versionOutput = execSync(`"${compilerPath}" --version`, { 
          encoding: 'utf8', 
          timeout: 3000,
          stdio: ['ignore', 'pipe', 'pipe'] // Don't inherit stdio to avoid potential hanging
        });
        if (versionOutput.trim() === version) {
          return compilerPath;
        } else {
          this.outputChannel.appendLine(`⚠️ Cached compiler version mismatch: expected ${version}, got ${versionOutput.trim()}`);
        }
      } catch (error) {
        this.outputChannel.appendLine(`⚠️ Cached compiler failed to run: ${error}`);
        // Remove the potentially corrupted cached compiler
        try {
          const compilerDir = path.dirname(path.dirname(compilerPath));
          if (compilerDir.includes('gren-compilers')) {
            require('fs').rmSync(compilerDir, { recursive: true, force: true });
            this.outputChannel.appendLine(`🗑️ Removed corrupted cached compiler at ${compilerDir}`);
          }
        } catch (cleanupError) {
          this.outputChannel.appendLine(`⚠️ Failed to cleanup corrupted compiler: ${cleanupError}`);
        }
      }
    }

    return null;
  }

  /**
   * Download and install a specific version of the Gren compiler
   */
  private async downloadGrenCompiler(version: string): Promise<string> {
    const compilerDir = path.join(this.context.globalStorageUri.fsPath, 'gren-compilers', version);
    const compilerPath = path.join(compilerDir, 'bin', 'gren');

    // Create the directory structure
    await fs.promises.mkdir(compilerDir, { recursive: true });

    // Show progress to user
    return vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: `Downloading Gren compiler v${version}`,
        cancellable: false
      },
      async (progress) => {
        progress.report({ increment: 0, message: 'Starting download...' });

        // Download using npm (since Gren is distributed via npm)
        const tempDir = path.join(os.tmpdir(), `gren-download-${Date.now()}`);
        await fs.promises.mkdir(tempDir, { recursive: true });

        try {
          progress.report({ increment: 30, message: 'Downloading from npm...' });
          
          // Use npm to download the specific version
          execSync(`npm pack gren-lang@${version}`, { 
            cwd: tempDir, 
            stdio: 'pipe',
            timeout: 30000 
          });

          progress.report({ increment: 60, message: 'Extracting package...' });

          // Extract the tarball
          const tarballName = `gren-lang-${version}.tgz`;
          const tarballPath = path.join(tempDir, tarballName);
          
          if (!fs.existsSync(tarballPath)) {
            throw new Error(`Downloaded tarball not found: ${tarballPath}`);
          }

          // Extract to a temp directory first
          const extractDir = path.join(tempDir, 'extracted');
          await fs.promises.mkdir(extractDir, { recursive: true });
          
          execSync(`tar -xzf "${tarballPath}" -C "${extractDir}"`, { timeout: 10000 });

          progress.report({ increment: 80, message: 'Installing compiler...' });

          // Copy the contents to the final location
          const packageDir = path.join(extractDir, 'package');
          if (!fs.existsSync(packageDir)) {
            throw new Error('Extracted package directory not found');
          }

          // Create bin directory
          const binDir = path.join(compilerDir, 'bin');
          await fs.promises.mkdir(binDir, { recursive: true });

          // Copy the entire package
          await this.copyRecursive(packageDir, compilerDir);

          // Create a wrapper script that points to the actual compiler
          const wrapperContent = this.createGrenWrapper(compilerDir);
          await fs.promises.writeFile(compilerPath, wrapperContent, { mode: 0o755 });

          progress.report({ increment: 100, message: 'Installation complete!' });

          // Verify the installation with proper error handling
          try {
            const versionOutput = execSync(`"${compilerPath}" --version`, { 
              encoding: 'utf8', 
              timeout: 5000,
              stdio: ['ignore', 'pipe', 'pipe'] // Don't inherit stdio
            });
            if (versionOutput.trim() !== version) {
              this.outputChannel.appendLine(`⚠️ Version mismatch: expected ${version}, got ${versionOutput.trim()}, but proceeding anyway`);
            } else {
              this.outputChannel.appendLine(`✅ Version verification successful: ${version}`);
            }
          } catch (verifyError) {
            this.outputChannel.appendLine(`⚠️ Could not verify compiler version, but installation appears complete: ${verifyError}`);
            // Don't throw here - the compiler might still work even if version check fails
          }

          return compilerPath;

        } finally {
          // Clean up temp directory
          try {
            await fs.promises.rm(tempDir, { recursive: true, force: true });
          } catch (error) {
            this.outputChannel.appendLine(`⚠️ Failed to clean up temp directory: ${error}`);
          }
        }
      }
    );
  }

  /**
   * Create a wrapper script for the Gren compiler
   */
  private createGrenWrapper(compilerDir: string): string {
    const isWindows = process.platform === 'win32';
    
    // First, let's find the actual compiler executable in the extracted package
    const possiblePaths = [
      path.join(compilerDir, 'bin', 'compiler'),
      path.join(compilerDir, 'bin', 'gren'),
      path.join(compilerDir, 'compiler'),
      path.join(compilerDir, 'gren'),
      path.join(compilerDir, 'index.js'),
    ];
    
    let compilerPath = path.join(compilerDir, 'bin', 'compiler'); // default fallback
    
    for (const possiblePath of possiblePaths) {
      if (fs.existsSync(possiblePath)) {
        compilerPath = possiblePath;
        this.outputChannel.appendLine(`✅ Found compiler executable at: ${compilerPath}`);
        break;
      }
    }
    
    if (!fs.existsSync(compilerPath)) {
      this.outputChannel.appendLine(`⚠️ Could not find compiler executable, using default: ${compilerPath}`);
      // List what files are actually in the directory for debugging
      try {
        const files = fs.readdirSync(compilerDir, { recursive: true });
        this.outputChannel.appendLine(`📁 Files in compiler directory: ${JSON.stringify(files)}`);
      } catch (error) {
        this.outputChannel.appendLine(`❌ Could not list files in compiler directory: ${error}`);
      }
    }
    
    if (isWindows) {
      // Windows batch script
      return `@echo off
if not exist "${compilerPath}" (
  echo Error: Gren compiler not found at ${compilerPath}
  exit /b 1
)
node "${compilerPath}" %*
`;
    } else {
      // Unix shell script
      return `#!/bin/bash
set -e
if [[ ! -f "${compilerPath}" ]]; then
  echo "Error: Gren compiler not found at ${compilerPath}" >&2
  exit 1
fi
exec node "${compilerPath}" "$@"
`;
    }
  }

  /**
   * Recursively copy directory contents
   */
  private async copyRecursive(src: string, dest: string): Promise<void> {
    const stat = await fs.promises.stat(src);
    
    if (stat.isDirectory()) {
      await fs.promises.mkdir(dest, { recursive: true });
      const files = await fs.promises.readdir(src);
      
      for (const file of files) {
        await this.copyRecursive(
          path.join(src, file),
          path.join(dest, file)
        );
      }
    } else {
      await fs.promises.copyFile(src, dest);
    }
  }

  /**
   * Find Gren compiler in system PATH as fallback
   */
  private findCompilerInPath(): string | null {
    try {
      const result = execSync('which gren', { 
        encoding: 'utf8', 
        timeout: 3000,
        stdio: ['ignore', 'pipe', 'pipe'] // Don't inherit stdio to avoid potential hanging
      });
      const compilerPath = result.trim();
      
      if (fs.existsSync(compilerPath)) {
        this.outputChannel.appendLine(`✅ Found compiler in PATH: ${compilerPath}`);
        
        // Quick verification that it works
        try {
          execSync(`"${compilerPath}" --version`, { 
            encoding: 'utf8', 
            timeout: 3000,
            stdio: ['ignore', 'pipe', 'pipe']
          });
          return compilerPath;
        } catch (verifyError) {
          this.outputChannel.appendLine(`⚠️ Compiler in PATH failed verification: ${verifyError}`);
        }
      }
    } catch (error) {
      this.outputChannel.appendLine(`❌ Gren compiler not found in PATH: ${error}`);
    }

    return null;
  }

  /**
   * Get the version of a compiler at a given path
   */
  async getCompilerVersion(compilerPath: string): Promise<string | null> {
    try {
      const versionOutput = execSync(`"${compilerPath}" --version`, { 
        encoding: 'utf8', 
        timeout: 5000 
      });
      return versionOutput.trim();
    } catch (error) {
      this.outputChannel.appendLine(`❌ Failed to get compiler version: ${error}`);
      return null;
    }
  }

  /**
   * Command to manually download/update the compiler
   */
  async downloadCompilerCommand(): Promise<void> {
    const requiredVersion = await this.getRequiredGrenVersion();
    
    if (!requiredVersion) {
      vscode.window.showErrorMessage('No gren.json found in workspace. Cannot determine required Gren version.');
      return;
    }

    try {
      await this.downloadGrenCompiler(requiredVersion);
      vscode.window.showInformationMessage(`Successfully downloaded Gren compiler v${requiredVersion}`);
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to download Gren compiler: ${error}`);
    }
  }

  /**
   * Command to show the current compiler version
   */
  async showCompilerVersionCommand(): Promise<void> {
    const compilerPath = await this.getCompilerPath();
    
    if (!compilerPath) {
      vscode.window.showErrorMessage('No Gren compiler found');
      return;
    }

    const version = await this.getCompilerVersion(compilerPath);
    if (version) {
      vscode.window.showInformationMessage(`Gren compiler version: ${version}\nPath: ${compilerPath}`);
    } else {
      vscode.window.showErrorMessage('Failed to get compiler version');
    }
  }
}