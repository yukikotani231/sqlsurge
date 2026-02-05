# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/claude-code) when working with this repository.

## Project Overview

sqlsurge is a SQL static analyzer that validates queries against schema definitions without requiring a database connection. It parses CREATE TABLE statements to build an in-memory schema catalog, then validates SQL queries (SELECT, INSERT, UPDATE, DELETE) against that catalog.

## Architecture

```
sqlsurge/
├── crates/
│   ├── sqlsurge-core/     # Core library (schema parsing, analysis engine)
│   │   ├── schema/        # Schema catalog and DDL parsing
│   │   ├── analyzer/      # Query validation and name resolution
│   │   ├── types/         # SQL type system
│   │   ├── dialect/       # SQL dialect abstraction
│   │   └── error.rs       # Diagnostic types
│   │
│   └── sqlsurge-cli/      # CLI binary
│       ├── args.rs        # CLI argument definitions (clap)
│       ├── output/        # Output formatters (human, JSON, SARIF)
│       └── main.rs        # Entry point
│
└── tests/fixtures/        # Test SQL files
```

### Key Components

1. **SchemaBuilder** (`schema/builder.rs`): Parses CREATE TABLE statements using sqlparser-rs and builds a `Catalog`
2. **Catalog** (`schema/catalog.rs`): In-memory representation of database schema (tables, columns, constraints)
3. **Analyzer** (`analyzer/mod.rs`): Entry point for query validation
4. **NameResolver** (`analyzer/resolver.rs`): Resolves table and column references, detects missing/ambiguous references
5. **SqlType** (`types/mod.rs`): Internal SQL type representation with compatibility checking

### Data Flow

```
Schema SQL → sqlparser → AST → SchemaBuilder → Catalog
                                                  ↓
Query SQL  → sqlparser → AST → Analyzer → NameResolver → Diagnostics
```

## Build & Test Commands

```bash
# Build
cargo build

# Run tests
cargo test

# Run with example
cargo run -- check --schema tests/fixtures/schema.sql tests/fixtures/valid_query.sql

# Check for errors
cargo run -- check --schema tests/fixtures/schema.sql tests/fixtures/invalid_query.sql
```

## Code Patterns

### Adding a New Diagnostic Rule

1. Add variant to `DiagnosticKind` in `error.rs`
2. Implement detection logic in `analyzer/resolver.rs` or create a new rule module
3. Add test case in `analyzer/mod.rs`

### Adding SQL Type Support

1. Add variant to `SqlType` enum in `types/mod.rs`
2. Update `SqlType::from_ast()` to handle the new sqlparser DataType
3. Update `SqlType::display_name()` for human-readable output
4. Update `is_compatible_with()` if needed for type coercion

### Adding CLI Options

1. Add field to appropriate struct in `args.rs` using clap derive macros
2. Handle the option in `main.rs`

## Dependencies

- **sqlparser** (0.53): SQL parsing (PostgreSQL dialect)
- **clap** (4.5): CLI argument parsing
- **miette** (7.4): Diagnostic rendering
- **thiserror** (2.0): Error type derivation
- **serde** (1.0): Serialization for JSON output
- **indexmap** (2.7): Ordered maps for deterministic output

## Testing Strategy

- Unit tests are colocated with modules (`#[cfg(test)] mod tests`)
- Integration tests use SQL fixtures in `tests/fixtures/`
- Test both positive cases (valid SQL) and negative cases (should produce diagnostics)

## Style Guidelines

- Follow Rust standard formatting (`cargo fmt`)
- Use `cargo clippy` for linting
- Prefer explicit error handling over `.unwrap()` in library code
- Document public APIs with doc comments
- Error messages should be actionable (include suggestions when possible)

## Current Limitations

- Only PostgreSQL dialect is fully supported
- Subquery column resolution is incomplete
- No support for VIEWs, functions, or stored procedures
- Type checking is basic (existence only, not full type inference)
