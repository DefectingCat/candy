# AGENTS.md

## Project Overview

**Candy** is a modern, lightweight web server written in Rust. Key features:
- Static file serving with directory listing support
- Reverse proxying with load balancing
- Lua scripting (optional feature)
- SSL/TLS encryption (HTTPS) with HTTP/2 support
- Auto-reload config on file change
- Multiple virtual hosts

## Build, Lint, and Test Commands

### Build

```bash
# Debug build
make build          # cargo build
make run            # cargo run

# Release build
make release        # cargo build --release
```

### Linting and Formatting

```bash
# Run linter (Clippy)
make lint           # cargo clippy

# Auto-format code (Rustfmt)
make format         # cargo fmt

# Auto-fix lint issues
make fix            # cargo fix + cargo fmt

# Check formatting without changes
cargo fmt --check
```

### Testing

```bash
# Run all tests
make test           # cargo test

# Run module tests
cargo test --package candy config       # Config module tests
cargo test --package candy config_watcher  # Config watcher tests

# Run single test by function name
cargo test test_settings_new --package candy
cargo test test_validate_config --package candy

# Run tests with verbose output
cargo test -v

# Run tests with specific options
cargo test -- --test-threads=1  # Single thread
cargo test -- --nocapture       # Show stdout

# Run tests with features
cargo test --features lua       # Run with Lua feature
```

### Development Workflow

```bash
# Watch mode with live reload
make dev            # cargo watch -x run

# Check compilation errors
make check          # cargo check

# Update dependencies
cargo update

# Add/remove dependencies
cargo add <dependency>
cargo remove <dependency>
```

## Code Style Guidelines

### General Rules

- **File encoding**: UTF-8
- **Line endings**: LF (Unix-style)
- **Trailing whitespace**: Must be trimmed
- **Final newline**: Required at end of file
- **Line length**: 80-100 characters (soft limit)
- **Indentation**: 4 spaces (enforced by .editorconfig)

### Import Order

Import statements must be grouped and sorted:

```rust
// 1. Standard library (std::*)
use std::path::Path;
use std::sync::Arc;

// 2. External dependencies (alphabetical)
use anyhow::Context;
use clap::Parser;
use dashmap::DashMap;
use serde::Deserialize;
use tokio::sync::Mutex;

// 3. Internal modules (crate::_, super::_, self::*)
use crate::config::Settings;
use crate::consts::{ARCH, COMMIT, VERSION};
use crate::http::{make_server, shutdown_servers};
```

### Naming Conventions

- **Variables/functions**: `snake_case`
- **Types/traits/enums**: `PascalCase`
- **Constants/static variables**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`
- **Lifetimes**: `'a`, `'b` (single lowercase)
- **Feature flags**: `snake_case`

### Type Annotations

- Use type inference for local variables
- Explicit types for:
  - Public API signatures
  - Complex type contexts
  - Where inference is ambiguous

```rust
// Good type annotation
pub fn load_config(path: &Path) -> anyhow::Result<Settings> {
    let config_str = std::fs::read_to_string(path)?;
    // ...
}
```

### Error Handling

- Use `anyhow::Result` for app-level errors
- Use `thiserror::Error` for structured errors
- Always use `?` operator instead of `unwrap()`/`expect()`
- Add context with `with_context()` for better error messages

```rust
// Good error handling
std::fs::read(path).with_context(|| format!("Failed to read {path:?}"))?;
```

### Memory Safety

- Prefer safe Rust constructs
- Document unsafe blocks with `// SAFETY:` comments
- Use `Arc` and `Mutex` for shared state management
- Avoid unnecessary allocations

### Documentation

- **Public API**: Must have documentation comments `///`
- **Module-level**: Use `//!` at the top of module files
- **Examples**: Include runnable examples in doc comments
- **Error messages**: Be descriptive and actionable

```rust
/// Validates configuration settings
///
/// # Examples
///
/// ```
/// let config = Config::new();
/// assert!(config.validate().is_ok());
/// ```
pub fn validate(&self) -> anyhow::Result<()> {
    // ...
}
```

### Logging

- Use `tracing` crate for logging
- Log levels: `trace`, `debug`, `info`, `warn`, `error`
- Include context in log messages
- Avoid logging sensitive information

## Cargo Configuration

```toml
[build]
rustflags = ["-C", "target-cpu=native"]

[package]
name = "candy"
version = "0.2.4"
edition = "2024"

[features]
default = ["all"]
all = ["lua"]
lua = ["dep:mlua"]

[profile.release]
opt-level = 3
strip = true
lto = true
panic = "abort"
codegen-units = 1
```

## Key Modules

- `src/main.rs`: Entry point, server lifecycle management
- `src/config.rs`: Configuration loading, validation, and struct definitions
- `src/http/mod.rs`: Axum server creation and route registration
- `src/utils/config_watcher.rs`: Config reloading on file change
- `src/lua_engine.rs`: Lua integration (optional feature)

## Git Integration

- Branch: `<type>/<short-description>` (feat|fix|docs|refactor|test|chore)
- Commits: Imperative mood, first line ≤ 50 chars, explain "why" in body
- Never commit secrets, credentials, or .env files

## Common Tasks

### Adding a New Feature

1. Create branch: `git checkout -b feat/your-feature`
2. Implement the feature
3. Add tests in the appropriate module
4. Run tests: `cargo test`
5. Run lint: `cargo clippy`
6. Format code: `cargo fmt`
7. Commit and create PR

### Debugging

- Run in debug mode with logging: `cargo run -- --config path/to/config.toml`
- Detailed logs: Set `log_level = "trace"` in config
- Show test output: `cargo test -- --nocapture`
