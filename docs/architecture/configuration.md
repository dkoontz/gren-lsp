# Configuration

## Environment Variables
```
GREN_COMPILER_PATH=/path/to/gren     # Gren compiler location
GREN_LSP_LOG_LEVEL=info              # Logging level
GREN_LSP_CACHE_SIZE=100              # Document cache size
GREN_LSP_MAX_DIAGNOSTICS=100         # Max diagnostics per file
GREN_LSP_COMPILE_TIMEOUT=5000        # Compilation timeout (ms)
```

## Runtime Configuration
- **Client Capabilities**: Adapt behavior based on client features
- **Project Configuration**: Read from gren.json or similar
- **User Preferences**: Handle client configuration changes
- **Feature Toggles**: Enable/disable features based on performance
