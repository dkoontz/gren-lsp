# Stories

## Story 4.1: Unit Testing Framework
**Description:** Set up comprehensive unit testing for all LSP server components.

**Acceptance Criteria:**
- [ ] Configure cargo test infrastructure
- [ ] Create test utilities and fixtures
- [ ] Write unit tests for parser integration
- [ ] Test symbol extraction logic
- [ ] Verify diagnostic generation
- [ ] Test completion algorithms
- [ ] Achieve 80% code coverage

**Technical Notes:**
- Use mockall for dependency mocking
- Create test fixtures from real Gren code
- Test error cases thoroughly

## Story 4.2: LSP Protocol Integration Tests
**Description:** Create integration tests that verify correct LSP protocol implementation.

**Acceptance Criteria:**
- [ ] Set up LSP client test harness
- [ ] Test initialize/shutdown sequence
- [ ] Verify document synchronization
- [ ] Test all request/response pairs
- [ ] Validate notification handling
- [ ] Check error responses
- [ ] Test concurrent requests

**Technical Notes:**
- Use tower-lsp testing utilities
- Simulate real client behavior
- Test protocol edge cases

## Story 4.3: Performance Benchmarking Suite
**Description:** Implement benchmarks to track and prevent performance regressions.

**Acceptance Criteria:**
- [ ] Set up criterion for benchmarking
- [ ] Benchmark file parsing performance
- [ ] Measure symbol indexing speed
- [ ] Test completion response times
- [ ] Profile memory usage
- [ ] Create performance regression tests
- [ ] Add benchmarks to CI pipeline

**Technical Notes:**
- Use realistic file sizes and counts
- Set performance targets from requirements
- Track trends over time

## Story 4.4: Fix Incremental Parsing Performance
**Description:** Restore and fix tree-sitter incremental parsing to improve performance during document edits.

**Acceptance Criteria:**
- [ ] Investigate tree-sitter incremental parsing state corruption issues
- [ ] Fix document synchronization with incremental updates
- [ ] Ensure parse tree consistency across edits
- [ ] Add comprehensive tests for incremental parsing edge cases
- [ ] Restore incremental parsing as default (currently disabled)
- [ ] Verify diagnostics update correctly with incremental parsing
- [ ] Benchmark performance improvement over full parsing

**Technical Notes:**
- Currently using full parsing as workaround for incremental parsing bugs
- Issue likely in `lsp-textdocument` integration or tree-sitter edit handling
- Need to investigate `TextDocumentContentChangeEvent` processing
- Consider using tree-sitter's edit tracking more directly

## Story 4.5: End-to-End VS Code Testing
**Description:** Create automated tests that verify the LSP works correctly with VS Code.

**Acceptance Criteria:**
- [ ] Set up VS Code extension test framework
- [ ] Test server installation and startup
- [ ] Verify basic editing operations
- [ ] Test go-to-definition navigation
- [ ] Validate error highlighting
- [ ] Check completion behavior
- [ ] Test with multiple file projects

**Technical Notes:**
- Use VS Code's test API
- Create representative test projects
- Test on multiple platforms

## Story 4.6: Fuzzing and Property Testing
**Description:** Implement fuzzing to find edge cases and ensure robustness.

**Acceptance Criteria:**
- [ ] Set up proptest for property testing
- [ ] Fuzz parser with random inputs
- [ ] Test protocol handling with malformed data
- [ ] Verify no panics on invalid input
- [ ] Test concurrent operation safety
- [ ] Document found edge cases

**Technical Notes:**
- Focus on input validation
- Test thread safety thoroughly
- Use AFL for deep fuzzing

## Story 4.7: Load Testing for Large Projects
**Description:** Ensure the LSP performs well with large Gren codebases.

**Acceptance Criteria:**
- [ ] Create large synthetic test projects
- [ ] Test with 1000+ file workspaces
- [ ] Measure indexing time for large codebases
- [ ] Verify memory usage stays reasonable
- [ ] Test incremental updates at scale
- [ ] Document performance characteristics

**Technical Notes:**
- Generate realistic project structures
- Test with real-world project sizes
- Identify bottlenecks

## Story 4.8: Error Recovery Testing
**Description:** Verify the LSP handles errors gracefully without crashing.

**Acceptance Criteria:**
- [ ] Test with malformed Gren syntax
- [ ] Verify handling of file system errors
- [ ] Test database corruption recovery
- [ ] Check network error handling
- [ ] Validate partial failure scenarios
- [ ] Ensure no data loss on crashes

**Technical Notes:**
- Inject failures systematically
- Test recovery mechanisms
- Verify error messages are helpful

## Story 4.9: Code Quality and Linting
**Description:** Establish and enforce code quality standards.

**Acceptance Criteria:**
- [ ] Configure clippy with strict lints
- [ ] Set up rustfmt configuration
- [ ] Add security audit to CI
- [ ] Implement code review checklist
- [ ] Document coding standards
- [ ] Create architectural tests

**Technical Notes:**
- Use pedantic clippy lints
- Enforce in CI pipeline
- Regular dependency updates

## Story 4.10: CI/CD Pipeline Setup
**Description:** Set up continuous integration pipeline for automated testing and quality checks.

**Acceptance Criteria:**
- [ ] Create GitHub Actions workflow for Rust
- [ ] Use `just ci` to run all checks in CI
- [ ] Run `just build` on all commits
- [ ] Run `just test` for unit tests
- [ ] Add `just lint` for clippy and fmt checks
- [ ] Cache dependencies for faster builds
- [ ] Set up matrix builds for multiple Rust versions
- [ ] Ensure CI uses same commands as local development
- [ ] Add build status badge to README
- [ ] Set up automated security audits

**Technical Notes:**
- Use just commands in CI for consistency with local development
- Implement proper caching strategies for faster builds
- Consider adding deployment automation for releases
