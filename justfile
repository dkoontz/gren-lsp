# Gren LSP Development Commands
# Run `just` to see available commands

# Default command - list all available recipes
default:
    @just --list

# Setup development environment
setup:
    @echo "Setting up Gren LSP development environment..."
    rustup component add rustfmt clippy
    cargo fetch
    @echo "Setup complete!"

# Build the project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run lints (format check + clippy)
lint:
    cargo fmt --check
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Auto-fix lint issues
fix:
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged

# Run all checks (format, lint, test)
check: fmt lint test

# Watch for changes and rebuild
watch:
    cargo watch -x build

# Watch and run tests
watch-test:
    cargo watch -x test

# Generate and open documentation
doc:
    cargo doc --open

# Generate documentation without opening
doc-build:
    cargo doc --no-deps

# Run benchmarks
bench:
    cargo bench

# Clean build artifacts
clean:
    cargo clean

# Install LSP binary locally
install:
    cargo install --path gren-lsp-server

# Uninstall LSP binary
uninstall:
    cargo uninstall gren-lsp

# Run the LSP server (for debugging)
run:
    cargo run --bin gren-lsp

# Run with debug logging
run-debug:
    RUST_LOG=gren_lsp=debug cargo run --bin gren-lsp

# Run with trace logging
run-trace:
    RUST_LOG=gren_lsp=trace cargo run --bin gren-lsp

# Check for outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Security audit
audit:
    cargo audit

# Create a new release (specify version with `just release 0.1.0`)
release version:
    @echo "Preparing release {{version}}..."
    cargo set-version {{version}}
    git add -A
    git commit -m "chore: release v{{version}}"
    git tag -a v{{version}} -m "Release version {{version}}"
    @echo "Release {{version}} prepared. Run 'git push --tags' to publish."

# Database commands
db-reset:
    rm -f gren-lsp-symbols.db
    @echo "Symbol database reset"

# Development utilities
todo:
    @rg "TODO|FIXME|HACK|XXX" --type rust

# Count lines of code
loc:
    @tokei

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

# Check if VS Code is running
_check_vscode_running:
    #!/usr/bin/env bash
    if pgrep -f "Visual Studio Code\|Code\|code" > /dev/null; then
        echo "⚠️  VS Code is still running. Please close VS Code completely before continuing."
        echo "   You can also run 'just vscode-kill' to force close all instances."
        exit 1
    fi

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

# Comprehensive environment cleanup (cleanup only, no installation)
clean-dev-env:
    @echo "🧹 Cleaning development environment..."
    # Force close VS Code instances
    just vscode-kill
    # Clean LSP server build artifacts
    cargo clean
    # Clean VS Code extension artifacts
    cd editor-extensions/vscode && rm -rf out/ node_modules/ tsconfig.tsbuildinfo *.vsix .vscode-test/
    # Remove symbol databases from all test projects
    find test-projects/ -name "gren-lsp-symbols.db" -delete || true
    find editor-extensions/vscode/ -name "gren-lsp-symbols.db" -delete || true
    # Clean LSP server logs and temp files
    rm -rf /tmp/gren-lsp/ || true
    rm -rf /var/folders/*/T/gren-lsp/ || true
    # Clean VS Code extension host cache (if accessible)
    rm -rf ~/.vscode/extensions/gren-lsp.gren-lsp-* || true
    @echo "✅ Development environment cleaned!"
    @echo "💡 Next steps:"
    @echo "   1. Wait 10 seconds to ensure VS Code is fully terminated"
    @echo "   2. Run 'just vscode-build-and-install' to rebuild and install"
    @echo "   3. Or run 'just vscode-dev-fresh' for automated setup"

# Build and install extension (separate from cleanup)
vscode-build-and-install: build _check_vscode_running
    @echo "🔄 Building and installing extension..."
    cd editor-extensions/vscode && npm install
    cd editor-extensions/vscode && npm run compile
    cd editor-extensions/vscode && npm run install-extension
    @echo "✅ Extension built and installed!"
    @echo "💡 You can now run 'code $(pwd)/test-projects/application' to test"

# Manual installation instructions (fallback)
vscode-install-manual: build
    @echo "📦 Building extension package..."
    cd editor-extensions/vscode && npm install
    cd editor-extensions/vscode && npm run compile
    cd editor-extensions/vscode && npm run package
    @echo "✅ Extension package created: editor-extensions/vscode/gren-lsp-1.0.0.vsix"
    @echo ""
    @echo "🔧 Manual installation steps:"
    @echo "   1. Make sure VS Code is completely closed"
    @echo "   2. Open terminal and run:"
    @echo "      code --install-extension $(pwd)/editor-extensions/vscode/gren-lsp-1.0.0.vsix --force"
    @echo "   3. Open the test project:"
    @echo "      code $(pwd)/test-projects/application"

# Fresh development setup with error handling
vscode-dev-fresh: clean-dev-env
    @echo "⏳ Waiting for VS Code to fully terminate..."
    sleep 10
    @echo "🔄 Setting up fresh development environment..."
    just vscode-build-and-install || just vscode-install-manual
    sleep 2
    code "$(pwd)/test-projects/application"
    @echo "✅ Fresh development environment ready!"
    @echo "💡 The extension should now work correctly with document changes"

vscode-install:
    cd editor-extensions/vscode && npm run install-extension

vscode-test: build
    cd editor-extensions/vscode && npm test

vscode-restart:
    pkill -f "code.*$(pwd)/test-projects/application" || true
    sleep 1
    code "$(pwd)/test-projects/application"

# Build LSP server and install VS Code extension
vscode-dev: build vscode-build vscode-install
    code "$(pwd)/test-projects/application"
    @echo "✅ LSP server built and VS Code extension installed!"
    @echo "💡 You can now open a Gren project in VS Code to test the extension"

# Aliases for common commands
alias b := build
alias t := test
alias r := run
alias c := check
alias f := fmt