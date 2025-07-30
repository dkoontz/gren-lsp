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

# Run all CI checks locally
ci: check audit
    @echo "All CI checks passed!"

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

vscode-clean:
    cd editor-extensions/vscode && rm -rf out/ node_modules/ tsconfig.tsbuildinfo *.vsix

vscode-install:
    cd editor-extensions/vscode && npm run install-extension

# Build LSP server and install VS Code extension
vscode-dev: build vscode-build vscode-install
    @echo "✅ LSP server built and VS Code extension installed!"
    @echo "💡 You can now open a Gren project in VS Code to test the extension"

# Aliases for common commands
alias b := build
alias t := test
alias r := run
alias c := check
alias f := fmt