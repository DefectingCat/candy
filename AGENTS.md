# AGENTS.md

This file provides instructions for agentic coding assistants working in this Rust repository.

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

# Run a specific test
cargo test --test <test_module_name>

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

### Rust-specific
- **Indentation**: 4 spaces (no tabs)
- **Import style**: Grouped as shown in Cargo.toml:
  1. Standard library
  2. External dependencies
  3. Internal modules
- **Naming conventions**:
  - Variables/functions: `snake_case`
  - Types: `PascalCase`
  - Constants: `SCREAMING_SNAKE_CASE`
- **Error handling**:
  - Use `anyhow` for application errors
  - Use `thiserror` for structured error types
  - Prefer `?` operator over `unwrap()`
  - Avoid `panic!` in production code
- **Type annotations**:
  - Use Rust's type inference where possible
  - Explicitly annotate public API signatures
  - Use `derive` macros for common traits (Debug, Clone, etc.)

### Documentation
- **Public API**: Must have doc comments `///`
- **Module-level**: Use `//!` at top of module files
- **Examples**: Include usage examples in doc comments

## Development Workflow

```bash
# Watch mode with live reload
make dev

# Check for compilation errors
make check
```

## Configuration

- Project configuration uses TOML format (`config.example.toml`)
- Copy `config.example.toml` to `config.toml` and customize

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

## Dependencies Management

- Dependencies are managed via Cargo.toml
- Always run `cargo update` after modifying dependencies
- Use exact version pins (">=0.1.0") for production dependencies

## Git Integration

- Branch naming: `<type>/<short-description>`
- Commit messages:
  - Use imperative mood ("Add feature" not "Added feature")
  - First line <= 50 characters
  - Body explains "why" not just "what"
```