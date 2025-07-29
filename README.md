# Gren Language Server Protocol (LSP) Implementation

A Language Server Protocol implementation for the [Gren programming language](https://gren-lang.org/), providing IDE features like code completion, go-to-definition, diagnostics, and more.

## Features (Planned)

- 🔍 Syntax highlighting and error diagnostics
- 📝 Code completion with type information
- 🔗 Go-to definition and find references
- 💡 Hover information with type signatures
- 🔧 Code actions and quick fixes
- 🏷️ Symbol search and navigation
- ♻️ Rename refactoring
- 📦 Import management

## Requirements

- Rust 1.75.0 or later
- [just](https://github.com/casey/just) command runner

## Quick Start

```bash
# Clone the repository
git clone https://github.com/dkoontz/gren-lsp
cd gren-lsp

# Setup development environment
just setup

# Build the project
just build

# Run tests
just test

# See all available commands
just
```

## Project Structure

```
gren-lsp/
├── gren-lsp-server/     # Main LSP server binary
├── gren-lsp-core/       # Core analysis engine
├── gren-lsp-protocol/   # LSP protocol handlers
├── docs/                # Documentation
│   └── epics/          # Development epics and stories
└── justfile            # Development commands
```

## Development

This project uses a workspace structure with three main crates:

- **gren-lsp-server**: The main executable that implements the LSP server
- **gren-lsp-core**: Core functionality including parsing, analysis, and symbol indexing
- **gren-lsp-protocol**: LSP protocol message handlers

### Common Commands

```bash
just build      # Build the project
just test       # Run all tests
just check      # Run all checks (format, lint, test)
just watch      # Watch for changes and rebuild
just doc        # Generate and open documentation
```

## Installation

### From Source

```bash
just install
```

This will install the `gren-lsp` binary to your Cargo bin directory.

## Editor Support

### VS Code

Extension coming soon!

### Other Editors

The LSP server communicates via stdio and is compatible with any editor that supports the Language Server Protocol.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to this project.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- The Gren language team for creating Gren
- The Rust LSP ecosystem for excellent tooling
- tree-sitter-gren contributors for the parsing grammar