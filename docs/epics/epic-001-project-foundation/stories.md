# Stories

## Story 1.1: Initialize Rust Project Structure
**Description:** Create the base Rust project with proper workspace organization for LSP server development.

**Acceptance Criteria:**
- [x] Create new Rust project using `cargo new gren-lsp --bin`
- [x] Set up workspace structure with separate crates for:
  - `gren-lsp-server` (main LSP server binary)
  - `gren-lsp-core` (core analysis logic)
  - `gren-lsp-protocol` (LSP protocol handling)
- [x] Configure Cargo.toml with proper metadata and workspace settings
- [x] Add `.gitignore` with Rust-specific patterns
- [x] Create initial directory structure following Rust best practices

**Technical Notes:**
- Use Rust 2021 edition
- Enable necessary features for async runtime
- Set up proper workspace dependencies

## Story 1.2: Configure Core Dependencies
**Description:** Install and configure all essential dependencies for LSP server implementation.

**Acceptance Criteria:**
- [x] Add tower-lsp 0.20.0 for LSP framework
- [x] Add tokio 1.32.0 with full features for async runtime
- [x] Add serde 1.0.185 with derive feature for JSON serialization
- [x] Add tracing 0.1.37 for structured logging
- [x] Configure lsp-textdocument 0.3.0 for document management
- [x] Verify all dependencies compile without conflicts

**Technical Notes:**
- Lock dependency versions for reproducibility
- Document why each dependency is needed
- Consider security audit of dependencies

## Story 1.3: Set Up Development Environment
**Description:** Create just recipes and configuration for consistent development environment setup.

**Acceptance Criteria:**
- [x] Install `just` as a project dependency/requirement
- [x] Create `justfile` with common development tasks:
  - `just setup` - Initialize development environment
  - `just build` - Build the project
  - `just test` - Run all tests
  - `just lint` - Run clippy and rustfmt
  - `just watch` - Watch for changes and rebuild
  - `just clean` - Clean build artifacts
- [x] Add rust-toolchain.toml specifying exact Rust version
- [x] ~~Configure VS Code workspace settings for Rust development~~ → Moved to Epic 5, Story 5.9
- [x] ~~Create `.env.example` for any configuration variables~~ → Moved to Epic 5, Story 5.9
- [x] ~~Add pre-commit hooks for code formatting (rustfmt)~~ → Moved to Epic 5, Story 5.9
- [x] ~~Document IDE setup for contributors~~ → Moved to Epic 5, Story 5.9

**Technical Notes:**
- Leverage just's cross-platform support for Windows/macOS/Linux
- Use just's built-in environment variable loading from .env
- Take advantage of just's recipe dependencies for task ordering

## Story 1.4: Implement Basic LSP Server Scaffold
**Description:** Create the minimal LSP server that can connect to clients and respond to initialization.

**Acceptance Criteria:**
- [x] Implement main.rs with tower-lsp server setup
- [x] Create basic LanguageServer trait implementation
- [x] Handle initialize/initialized protocol handshake
- [x] Implement shutdown and exit handlers
- [x] Add basic logging for all LSP messages
- [x] Server starts and accepts connections via stdio

**Technical Notes:**
- Focus on protocol compliance over features
- Ensure proper error handling from the start
- Add comprehensive logging for debugging

## Story 1.5: Configure SQLite for Symbol Storage
**Description:** Set up SQLite database infrastructure for persistent symbol indexing.

**Acceptance Criteria:**
- [x] Add rusqlite 1.29.0 dependency
- [x] Create database schema for symbols, files, and references
- [x] Implement database initialization and migration system
- [x] ~~Add connection pooling for concurrent access~~ → Moved to Epic 5, Story 5.10
- [x] Create basic CRUD operations for symbol storage
- [x] ~~Ensure database is created in appropriate user data directory~~ → Moved to Epic 5, Story 5.10

**Technical Notes:**
- Use embedded SQLite for zero-configuration deployment
- Design schema for efficient symbol queries
- Plan for future schema migrations

## Story 1.6: Create Initial README and Documentation
**Description:** Provide clear documentation for project setup and contribution.

**Acceptance Criteria:**
- [x] Create comprehensive README.md with:
  - Project overview and goals
  - Installation instructions
  - Development setup guide
  - Architecture overview
  - Contribution guidelines
- [x] Add LICENSE file (choose appropriate open-source license)
- [x] ~~Create CONTRIBUTING.md with development workflow~~ → Moved to Epic 5, Story 5.9
- [x] ~~Add architecture decision records (ADR) directory~~ → Moved to Epic 5, Story 5.9
- [x] ~~Document project structure and module organization~~ → Moved to Epic 5, Story 5.9

**Technical Notes:**
- Keep documentation close to code
- Use clear examples in setup instructions
- Link to Gren language resources

## Story 1.7: Set Up Basic CI Pipeline
**Description:** Configure continuous integration for automated testing and code quality checks using just commands.

**Acceptance Criteria:**
- [x] ~~Create GitHub Actions workflow for Rust~~ → Moved to Epic 4, Story 4.9
- [x] ~~Use `just ci` to run all checks in CI~~ → Moved to Epic 4, Story 4.9
- [x] ~~Run `just build` on all commits~~ → Moved to Epic 4, Story 4.9
- [x] ~~Run `just test` for unit tests~~ → Moved to Epic 4, Story 4.9
- [x] ~~Add `just lint` for clippy and fmt checks~~ → Moved to Epic 4, Story 4.9
- [x] ~~Cache dependencies for faster builds~~ → Moved to Epic 4, Story 4.9
- [x] ~~Set up matrix builds for multiple Rust versions~~ → Moved to Epic 4, Story 4.9
- [x] ~~Ensure CI uses same commands as local development~~ → Moved to Epic 4, Story 4.9

**Technical Notes:**
- Use just commands in CI for consistency with local development
- Add badge to README showing build status
- Consider adding security audit in future

## Story 1.8: Create Comprehensive Justfile
**Description:** Develop a complete justfile with all common development and maintenance tasks.

**Acceptance Criteria:**
- [x] Create advanced just recipes:
  - `just check` - Run all checks (fmt, clippy, test)
  - `just fix` - Auto-fix formatting and clippy warnings
  - `just bench` - Run performance benchmarks
  - `just doc` - Generate and open documentation
  - `just release` - Prepare a new release
  - `just install` - Install the LSP locally
  - `just uninstall` - Remove local installation
  - `just ci` - Run all CI checks locally
- [x] Add recipe aliases for common workflows
- [x] ~~Implement recipe dependencies (e.g., test depends on build)~~ → Moved to Epic 5, Story 5.9
- [x] Add helpful descriptions for each recipe
- [x] Support parameterized recipes where needed
- [x] ~~Document all recipes in README~~ → Moved to Epic 5, Story 5.9

**Technical Notes:**
- Use just's `@` prefix to suppress command echo when appropriate
- Leverage just's ability to run recipes from any subdirectory
- Consider using just's ability to invoke recipes in different languages
- Take advantage of just's error handling and informative messages

## Story 1.9: Set Up LSP Testing Infrastructure
**Description:** Create testing infrastructure to validate LSP protocol communication and enable Epic 2 development.

**Acceptance Criteria:**
- [x] Add tower-lsp testing utilities to dev dependencies
- [x] Create basic LSP client test harness for protocol testing
- [x] Implement test utilities for LSP message creation and validation
- [x] Add helper functions for testing initialize/shutdown sequence
- [x] Create mock LSP client for testing server responses
- [x] Add integration test that validates basic LSP protocol compliance
- [x] Ensure test infrastructure can simulate document events (open/change/close)
- [x] Add logging and debugging support for test scenarios

**Technical Notes:**
- Use tower-lsp's built-in testing capabilities where available
- Consider lsp-test-client or similar tools for integration testing
- Focus on testing LSP protocol compliance rather than language-specific features
- Ensure test client can send/receive all basic LSP message types
- Design for easy expansion in Epic 2 when adding language features

## Story 1.10: Create Basic VS Code Extension for Manual Testing
**Description:** Create a minimal VS Code extension to enable manual testing of LSP functionality after each epic.

**Acceptance Criteria:**
- [x] Create `editors/vscode/` directory structure
- [x] Create basic `package.json` with extension metadata
- [x] Implement minimal extension activation that starts the LSP server
- [x] Configure language client to connect to our LSP server via stdio
- [x] Add basic language configuration (file associations, syntax highlighting)
- [x] Create extension launch configuration for VS Code development
- [x] Test extension can initialize and communicate with LSP server
- [x] Document how to install and test the extension locally

**Technical Notes:**
- Keep extension minimal - focus on LSP client functionality
- Use VS Code Language Client library for LSP communication
- Set up for local development and testing, not marketplace distribution
- Ensure extension can be easily rebuilt and tested after code changes
- Configure proper logging to debug LSP communication issues
