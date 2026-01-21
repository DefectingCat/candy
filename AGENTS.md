# AGENTS.md

This file provides instructions for agentic coding assistants working in this Rust repository.

## Project Overview

**Candy** is a modern, lightweight web server written in Rust. It supports:
- Static file serving
- Reverse proxying
- Lua scripting (optional feature)
- SSL/TLS encryption
- Configuration reload on file change
- Multiple virtual hosts

## Build and Test Commands

```bash
# Build project (debug)
make build

# Build release version
make release

# Run application
make run

# Run all tests
make test

# Run config module tests specifically
cargo test --package candy config

# Run config watcher tests
cargo test --package candy config_watcher

# Run a specific test function
cargo test test_settings_new --package candy

# Run tests with verbose output
cargo test -v

# Clean build artifacts
make clean
```

## Linting and Formatting

```bash
# Run linter (Clippy)
make lint

# Auto-format code (Rustfmt)
make format

# Auto-fix lint issues
make fix

# Check for formatting issues without making changes
cargo fmt --check
```

## Cross-Compilation Targets

The project supports cross-compilation using `cross`. Available targets:

```bash
make linux-musl         # x86_64 Linux (musl)
make aarch64-linux-musl # ARM64 Linux (musl)
make aarch64-android    # ARM64 Android
make linux-gnu          # x86_64 Linux (GNU)
make windows-gnu        # x86_64 Windows
make freebsd            # x86_64 FreeBSD
make loongarch          # LoongArch Linux
```

## Code Style Guidelines

### General
- **File encoding**: UTF-8
- **Line endings**: LF (Unix-style)
- **Trailing whitespace**: Must be trimmed
- **Final newline**: Required at end of file
- **Line length**: Aim for 80-100 characters (soft limit)

### Rust-specific
- **Indentation**: 4 spaces (no tabs) - enforced by .editorconfig
- **Import style**: Group and order as follows:
  1. Standard library (std::*)
  2. External dependencies (alphabetical)
  3. Internal modules (crate::*, super::*, self::*)
  - Use `use` statements for specific items, avoid glob imports
- **Naming conventions**:
  - Variables/functions: `snake_case`
  - Types/traits/enums: `PascalCase`
  - Constants/static variables: `SCREAMING_SNAKE_CASE`
  - Modules: `snake_case`
  - Lifetimes: `'a`, `'b` (single lowercase letter)
- **Error handling**:
  - Use `anyhow::Result` for application-level errors
  - Use `thiserror::Error` for structured error types
  - Prefer `?` operator over `unwrap()`/`expect()`
  - Avoid `panic!` in production code (use only for unrecoverable errors)
  - Provide context with `with_context()` for better error messages
- **Type annotations**:
  - Use Rust's type inference where possible
  - Explicitly annotate public API signatures
  - Use `derive` macros for common traits (Debug, Clone, PartialEq, Eq)
- **Memory safety**:
  - Prefer safe Rust over unsafe Rust
  - Document unsafe blocks with `// SAFETY:` comments

### Documentation
- **Public API**: Must have doc comments `///`
- **Module-level**: Use `//!` at top of module files
- **Examples**: Include runnable examples in doc comments
- **Error messages**: Be descriptive and actionable
- **Internal documentation**: Use `//!` for module-level docs, `///` for internal items

## Development Workflow

```bash
# Watch mode with live reload
make dev

# Check for compilation errors
make check

# Update dependencies
cargo update

# Add a new dependency
cargo add <dependency_name>

# Remove a dependency
cargo remove <dependency_name>

# Run with custom config file
cargo run -- --config path/to/config.toml
```

## Configuration

- Project configuration uses TOML format (`config.example.toml`)
- Copy `config.example.toml` to `config.toml` and customize
- Never commit `config.toml` to version control
- Configuration file is automatically reloaded when changed

## Performance Optimization

Release builds include these optimizations:
```toml
[profile.release]
opt-level = 3
strip = true
lto = true
panic = "abort"
codegen-units = 1
```

## Git Integration

- Branch naming: `<type>/<short-description>`
  - Types: feat, fix, docs, refactor, test, chore
- Commit messages:
  - Use imperative mood ("Add feature" not "Added feature")
  - First line <= 50 characters
  - Body explains "why" not just "what"
  - Reference issues/PRs where relevant
- Never commit secrets or sensitive information

## Key Modules

### Main Entry Point
- **src/main.rs**: Initializes logger, loads config, starts servers, and watches for config changes

### Configuration
- **src/config.rs**: Defines configuration structure, validation, and loading from TOML files

### HTTP Server
- **src/http/mod.rs**: Core server implementation using Axum
- **src/http/handler.rs**: Request handlers for static files, proxy, and Lua scripts
- **src/http/router.rs**: Route matching and dispatch logic

### Utilities
- **src/utils/config_watcher.rs**: Monitors configuration file for changes and reloads config
- **src/utils/init_logger.rs**: Initializes tracing logger
- **src/utils/mime_types.rs**: MIME type detection for static files

### Lua Engine (Optional)
- **src/lua_engine.rs**: Lua script execution context (enabled with "lua" feature)

### Error Handling
- **src/error.rs**: Custom error types and conversion functions