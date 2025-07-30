# Introduction

This document outlines the overall project architecture for Gren LSP, including backend systems, shared services, and non-UI specific concerns. Its primary goal is to serve as the guiding architectural blueprint for AI-driven development, ensuring consistency and adherence to chosen patterns and technologies.

**Gren Language Characteristics:**
Gren is a pure functional programming language forked from Elm with unique characteristics that significantly influence LSP architecture:
- **Immutable data structures** - Arrays as primary sequential type (not lists)
- **No runtime exceptions** - All errors handled through Maybe/Result types
- **Small, predictable syntax** - Minimal language constructs with clear semantics
- **Planned parametric modules** - Future module system changes
- **Referential transparency** - Functions always return same output for same input

**Relationship to Frontend Architecture:**
This project is primarily a Language Server Protocol implementation with minimal UI components (limited to VS Code extension and configuration interfaces). The core architecture focuses on the LSP server, tree-sitter integration, and editor communication protocols optimized for functional programming paradigms.

## Starter Template or Existing Project

**Decision: Custom Implementation from Scratch**

After reviewing the PRD requirements and the unique nature of LSP implementations, no existing starter template will be used. The Gren LSP requires:

- Deep integration with the Gren compiler's AST and type system
- Custom protocol handling for LSP communication
- Specialized parsing and analysis capabilities
- Performance-critical real-time processing

While LSP libraries exist for various languages, the tight coupling required with Gren's compiler and the need for language-specific optimizations make a custom implementation more appropriate. This approach allows for:

- Optimal performance tuning for Gren's functional paradigms
- Direct compiler integration without abstraction layers
- Custom caching and incremental compilation strategies
- Specialized handling of Gren's type system and module structure

## Change Log

| Date | Version | Description | Author |
|------|---------|-------------|---------|
| 2025-07-28 | 1.0 | Initial architecture document | Claude |
