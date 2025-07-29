# Gren LSP Extension for VS Code

VS Code extension providing Language Server Protocol support for the Gren programming language.

## Features

- Syntax highlighting for `.gren` files
- Language Server Protocol integration
- Basic language configuration (comments, brackets, indentation)

## Local Development and Testing

### Prerequisites

1. Node.js and npm
2. VS Code
3. Rust toolchain (for building the LSP server)

### Setup

1. Build the Gren LSP server:
   ```bash
   cd ../../  # Go to project root
   just build
   ```

2. Install extension dependencies:
   ```bash
   npm install
   ```

3. Compile the extension:
   ```bash
   npm run compile
   ```

### Testing the Extension

1. Open VS Code in this directory:
   ```bash
   code .
   ```

2. Press `F5` to launch the Extension Development Host

3. In the new VS Code window, open the `test.gren` file

4. The extension should:
   - Recognize the `.gren` file type
   - Apply syntax highlighting
   - Start the LSP server (check the "Gren LSP" output channel)

### Configuration

The extension supports the following settings:

- `grenLsp.serverPath`: Path to the Gren LSP server binary (defaults to workspace relative path)
- `grenLsp.trace.server`: LSP communication tracing level (`off`, `messages`, `verbose`)

### Troubleshooting

- Check the "Gren LSP" output channel for server logs
- Ensure the Rust LSP server is compiled (`just build` in project root)
- Verify the server path in settings matches your built binary location

## Development Workflow

1. Make changes to the Rust LSP server
2. Rebuild with `just build` from project root
3. Restart the Extension Development Host (`Ctrl+Shift+F5` or `Cmd+Shift+F5`)
4. Test with `.gren` files

## Project Structure

```
.
├── .vscode
│   └── launch.json         // VS Code launch configuration
├── client
│   ├── package.json        // Client dependencies and metadata
│   ├── src
│   │   └── extension.ts    // Extension activation and LSP client setup
│   └── tsconfig.json       // TypeScript configuration
├── language-configuration.json  // Gren language configuration
├── package.json            // Extension manifest
├── test.gren              // Sample Gren file for testing
└── tsconfig.json          // Root TypeScript configuration
```

## Future Improvements

This is a minimal extension for Epic 1. Future epics will add:
- Enhanced syntax highlighting
- Code completion
- Hover information
- Go-to-definition
- Error diagnostics
- Code formatting