# AGENTS.md

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

# Run module tests
cargo test --package candy config  # Config module
cargo test --package candy config_watcher

# Run single test by function name
cargo test test_settings_new --package candy
cargo test test_validate_config --package candy

# Run specific test module
cargo test -p candy --test config_tests

# Run tests with verbose output
cargo test -v

# Run tests with specific filter
cargo test -- --test-threads=1 # Single thread
cargo test -- --nocapture      # Show stdout

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

# Check for formatting issues
cargo fmt --check
```

## Cross-Compilation Targets

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
- **Line length**: 80-100 characters (soft limit)

### Rust-specific

#### Import Order
1. Standard library (std::*)
2. External dependencies (alphabetical)
3. Internal modules (crate::*, super::*, self::*)
```rust
// Good import example
use std::path::Path;
use anyhow::Context;
use serde::Deserialize;
use crate::config::Settings;
```

#### Naming Conventions
- Variables/functions: `snake_case`
- Types/traits/enums: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`
- Lifetimes: `'a`, `'b` (single lowercase)

#### Type Annotations
- Use inference for local variables
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

#### Error Handling
- Use `anyhow::Result` for app errors
- Use `thiserror::Error` for structured errors
- Always use `?` operator instead of `unwrap()`/`expect()`
- Add context with `with_context()`:
```rust
std::fs::read(path).with_context(|| format!("Failed to read {path:?}"))?;
```

### Memory Safety
- Prefer safe Rust constructs
- Document unsafe blocks:
```rust
// SAFETY: Buffer size verified before access
unsafe { *ptr = value; }
```

## Documentation Guidelines

### Code Comments
- `///` for public API documentation
- `//!` for module-level documentation
- `//` for implementation comments

### Examples
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

### Error Messages
- Include actionable information
- Suggest solutions when possible

## Security Best Practices
- Never log secrets or credentials
- Validate all external inputs
- Use constant-time comparisons for sensitive data
- Avoid hardcoded credentials

## Development Workflow

```bash
# Watch mode with live reload
make dev

# Check for compilation errors
make check

# Dependency management
cargo update
cargo add <dependency>
cargo remove <dependency>

# Run with custom config
cargo run -- --config path/to/config.toml
```

## Key Modules

- `src/main.rs`: Entry point
- `src/config.rs`: Configuration loading
- `src/http/mod.rs`: Axum server
- `src/utils/config_watcher.rs`: Config reloading
- `src/lua_engine.rs`: Lua integration

## Performance Optimization

Release profile:
```toml
[profile.release]
opt-level = 3
strip = true
lto = true
panic = "abort"
codegen-units = 1
```

## Git Integration

- Branch: `<type>/<short-description>` (feat|fix|docs|refactor|test|chore)
- Commits: 
  - Imperative mood ("Add", "Fix", "Update")
  - First line ≤ 50 chars
  - Explain "why" in body
- Never commit: Secrets, credentials, `.env` files