#!/bin/bash

# Epic 6 Story 3: Production Validation Execution Script
# 
# Executes the complete production validation suite and generates all reports

set -e  # Exit on any error

echo "🚀 Epic 6 Story 3: Production Readiness Validation"
echo "=================================================="
echo ""

# Check if we're in the right directory
if [ ! -f "run-production-validation.js" ]; then
    echo "❌ Error: Must be run from dev-tools/production-validation directory"
    exit 1
fi

# Check if Node.js is available
if ! command -v node &> /dev/null; then
    echo "❌ Error: Node.js is required but not installed"
    exit 1
fi

# Check if required project structure exists
echo "🔍 Checking project structure..."

if [ ! -f "../../lsp-server/target/debug/gren-lsp" ]; then
    echo "⚠️  LSP server binary not found - attempting to build..."
    cd ../../
    if command -v just &> /dev/null; then
        just build
    else
        echo "❌ Error: 'just' command not found. Please run 'cargo build' in lsp-server directory"
        exit 1
    fi
    cd dev-tools/production-validation
fi

if [ ! -f "../../editor-extensions/vscode/package.json" ]; then
    echo "❌ Error: VS Code extension not found"
    exit 1
fi

if [ ! -d "../test-data/gren-example-projects" ]; then
    echo "❌ Error: Test data not found"
    exit 1
fi

echo "✅ Project structure verified"
echo ""

# Execute the master validation
echo "⚡ Starting Epic 6 Story 3 Master Production Validation..."
echo ""

node run-production-validation.js

# Check if validation completed successfully
if [ $? -eq 0 ]; then
    echo ""
    echo "🎉 Epic 6 Story 3 Production Validation COMPLETED!"
    echo ""
    echo "📄 Generated Reports:"
    echo "  - docs/epic-6-story-3-master-validation-report.md"
    echo "  - docs/epic-6-story-3-executive-summary.md"
    echo "  - docs/production-readiness-assessment.md"
    echo "  - docs/end-to-end-test-scenarios.md"
    echo "  - docs/stability-test-results.md"
    echo "  - docs/performance-monitoring-results.md"
    echo "  - docs/cross-platform-compatibility.md"
    echo ""
    echo "🎯 Epic 6 Story 3 implementation COMPLETE!"
else
    echo ""
    echo "❌ Epic 6 Story 3 Production Validation FAILED!"
    echo "Check the output above for error details."
    exit 1
fi