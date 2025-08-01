# Gren LSP Development Commands
# Run `just` to see available commands

# Default command - list all available recipes
default:
    @just --list



# VS Code specific commands
vscode-build:
    cd editor-extensions/vscode && npm run compile

vscode-package:
    cd editor-extensions/vscode && npm run package

vscode-watch:
    cd editor-extensions/vscode && npm run watch

# Clean VS Code extension build artifacts
vscode-clean:
    cd editor-extensions/vscode && rm -rf out/ node_modules/ tsconfig.tsbuildinfo *.vsix

# Force kill all VS Code processes
vscode-kill:
    @echo "🚫 Force closing all VS Code instances..."
    -pkill -f "Visual Studio Code" || true
    -pkill -f "Code Helper" || true
    -pkill -f "Code" || true
    -pkill -f "code" || true
    -pkill -f "Electron" || true
    @echo "⏳ Waiting for processes to terminate..."
    sleep 5
    @echo "✅ VS Code processes terminated"

vscode-install:
    cd editor-extensions/vscode && npm run install-extension

# Build LSP server and install VS Code extension
vscode-dev: vscode-build vscode-install
    code "$(pwd)/test-projects/application"
    @echo "✅ LSP server built and VS Code extension installed!"
    @echo "💡 You can now open a Gren project in VS Code to test the extension"

# Aliases for common commands
alias b := build
alias t := test
alias r := run
alias c := check
alias f := fmt
