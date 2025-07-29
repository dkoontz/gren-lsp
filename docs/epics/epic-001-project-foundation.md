# Epic 1: Project Foundation & Setup

## Epic Overview
Establish the foundational infrastructure for the Gren LSP server, including project setup, development environment configuration, and core dependencies installation.

## Epic Goals
- Create a well-structured Rust project for LSP development
- Set up development and build environments
- Install and configure all core dependencies
- Establish basic CI/CD pipeline
- Create initial project documentation

## Success Criteria
- Rust project compiles successfully
- All dependencies are installed and properly configured
- Development environment is reproducible
- Basic CI pipeline runs tests on commits
- README provides clear setup instructions

## Dependencies
- None (this is the foundational epic)

## Stories

### Story 1.1: Initialize Rust Project Structure
**Description:** Create the base Rust project with proper workspace organization for LSP server development.

**Acceptance Criteria:**
- [ ] Create new Rust project using `cargo new gren-lsp --bin`
- [ ] Set up workspace structure with separate crates for:
  - `gren-lsp-server` (main LSP server binary)
  - `gren-lsp-core` (core analysis logic)
  - `gren-lsp-protocol` (LSP protocol handling)
- [ ] Configure Cargo.toml with proper metadata and workspace settings
- [ ] Add `.gitignore` with Rust-specific patterns
- [ ] Create initial directory structure following Rust best practices

**Technical Notes:**
- Use Rust 2021 edition
- Enable necessary features for async runtime
- Set up proper workspace dependencies

### Story 1.2: Configure Core Dependencies
**Description:** Install and configure all essential dependencies for LSP server implementation.

**Acceptance Criteria:**
- [ ] Add tower-lsp 0.20.0 for LSP framework
- [ ] Add tokio 1.32.0 with full features for async runtime
- [ ] Add serde 1.0.185 with derive feature for JSON serialization
- [ ] Add tracing 0.1.37 for structured logging
- [ ] Configure lsp-textdocument 0.3.0 for document management
- [ ] Verify all dependencies compile without conflicts

**Technical Notes:**
- Lock dependency versions for reproducibility
- Document why each dependency is needed
- Consider security audit of dependencies

### Story 1.3: Set Up Development Environment
**Description:** Create just recipes and configuration for consistent development environment setup.

**Acceptance Criteria:**
- [ ] Install `just` as a project dependency/requirement
- [ ] Create `justfile` with common development tasks:
  - `just setup` - Initialize development environment
  - `just build` - Build the project
  - `just test` - Run all tests
  - `just lint` - Run clippy and rustfmt
  - `just watch` - Watch for changes and rebuild
  - `just clean` - Clean build artifacts
- [ ] Add rust-toolchain.toml specifying exact Rust version
- [ ] Configure VS Code workspace settings for Rust development
- [ ] Create `.env.example` for any configuration variables
- [ ] Add pre-commit hooks for code formatting (rustfmt)
- [ ] Document IDE setup for contributors

**Technical Notes:**
- Leverage just's cross-platform support for Windows/macOS/Linux
- Use just's built-in environment variable loading from .env
- Take advantage of just's recipe dependencies for task ordering

### Story 1.4: Implement Basic LSP Server Scaffold
**Description:** Create the minimal LSP server that can connect to clients and respond to initialization.

**Acceptance Criteria:**
- [ ] Implement main.rs with tower-lsp server setup
- [ ] Create basic LanguageServer trait implementation
- [ ] Handle initialize/initialized protocol handshake
- [ ] Implement shutdown and exit handlers
- [ ] Add basic logging for all LSP messages
- [ ] Server starts and accepts connections via stdio

**Technical Notes:**
- Focus on protocol compliance over features
- Ensure proper error handling from the start
- Add comprehensive logging for debugging

### Story 1.5: Configure SQLite for Symbol Storage
**Description:** Set up SQLite database infrastructure for persistent symbol indexing.

**Acceptance Criteria:**
- [ ] Add rusqlite 1.29.0 dependency
- [ ] Create database schema for symbols, files, and references
- [ ] Implement database initialization and migration system
- [ ] Add connection pooling for concurrent access
- [ ] Create basic CRUD operations for symbol storage
- [ ] Ensure database is created in appropriate user data directory

**Technical Notes:**
- Use embedded SQLite for zero-configuration deployment
- Design schema for efficient symbol queries
- Plan for future schema migrations

### Story 1.6: Create Initial README and Documentation
**Description:** Provide clear documentation for project setup and contribution.

**Acceptance Criteria:**
- [ ] Create comprehensive README.md with:
  - Project overview and goals
  - Installation instructions
  - Development setup guide
  - Architecture overview
  - Contribution guidelines
- [ ] Add LICENSE file (choose appropriate open-source license)
- [ ] Create CONTRIBUTING.md with development workflow
- [ ] Add architecture decision records (ADR) directory
- [ ] Document project structure and module organization

**Technical Notes:**
- Keep documentation close to code
- Use clear examples in setup instructions
- Link to Gren language resources

### Story 1.7: Set Up Basic CI Pipeline
**Description:** Configure continuous integration for automated testing and code quality checks using just commands.

**Acceptance Criteria:**
- [ ] Create GitHub Actions workflow for Rust
- [ ] Use `just ci` to run all checks in CI
- [ ] Run `just build` on all commits
- [ ] Run `just test` for unit tests
- [ ] Add `just lint` for clippy and fmt checks
- [ ] Cache dependencies for faster builds
- [ ] Set up matrix builds for multiple Rust versions
- [ ] Ensure CI uses same commands as local development

**Technical Notes:**
- Use just commands in CI for consistency with local development
- Add badge to README showing build status
- Consider adding security audit in future

### Story 1.8: Create Comprehensive Justfile
**Description:** Develop a complete justfile with all common development and maintenance tasks.

**Acceptance Criteria:**
- [ ] Create advanced just recipes:
  - `just check` - Run all checks (fmt, clippy, test)
  - `just fix` - Auto-fix formatting and clippy warnings
  - `just bench` - Run performance benchmarks
  - `just doc` - Generate and open documentation
  - `just release` - Prepare a new release
  - `just install` - Install the LSP locally
  - `just uninstall` - Remove local installation
  - `just ci` - Run all CI checks locally
- [ ] Add recipe aliases for common workflows
- [ ] Implement recipe dependencies (e.g., test depends on build)
- [ ] Add helpful descriptions for each recipe
- [ ] Support parameterized recipes where needed
- [ ] Document all recipes in README

**Technical Notes:**
- Use just's `@` prefix to suppress command echo when appropriate
- Leverage just's ability to run recipes from any subdirectory
- Consider using just's ability to invoke recipes in different languages
- Take advantage of just's error handling and informative messages

## Story Sequence
1. Story 1.1 (Initialize Project) → 
2. Story 1.2 (Configure Dependencies) → 
3. Story 1.3 (Dev Environment) & Story 1.8 (Comprehensive Justfile) →
4. Story 1.4 (LSP Scaffold) & Story 1.5 (SQLite Setup) in parallel →
5. Story 1.6 (Documentation) & Story 1.7 (CI Pipeline) in parallel

## Risks and Mitigations
- **Risk:** Dependency version conflicts
  - *Mitigation:* Lock all versions, test compatibility thoroughly
- **Risk:** Platform-specific issues in development setup
  - *Mitigation:* Test on multiple platforms, provide platform-specific instructions
- **Risk:** SQLite performance for large workspaces
  - *Mitigation:* Design efficient schema, add indexing, plan for optimization

## Definition of Done
- All stories completed and acceptance criteria met
- Code compiles without warnings
- Basic tests pass
- CI pipeline is green
- Documentation is complete and accurate
- Another developer can set up the project following README