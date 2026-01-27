# AGENTS.md

## Project Overview

**Candy** is a modern, lightweight web server written in Rust. It supports:

- Static file serving with directory listing support
- Reverse proxying to backend servers
- Lua scripting (optional feature)
- SSL/TLS encryption (HTTPS)
- HTTP/2 support
- Auto-reload config on file change
- Multiple virtual hosts
- Single binary deployment

## Project Structure

```
/home/xfy/Developer/candy/
├── src/
│   ├── main.rs                # Entry point with server initialization
│   ├── config.rs              # Configuration loading and validation
│   ├── cli.rs                 # Command-line interface parsing
│   ├── consts.rs              # Constant definitions (version, arch, commit info)
│   ├── error.rs               # Custom error types and error handling
│   ├── http/
│   │   ├── mod.rs             # HTTP server creation and shutdown
│   │   ├── serve.rs           # Static file serving handler
│   │   ├── reverse_proxy.rs   # Reverse proxy handler
│   │   ├── lua/               # Lua scripting handler (optional)
│   │   ├── redirect.rs        # HTTP redirect handler
│   │   └── error.rs           # HTTP error handling
│   ├── middlewares/           # HTTP middleware (version, headers, logging)
│   ├── utils/
│   │   ├── mod.rs             # Utility functions
│   │   ├── config_watcher.rs  # Configuration file watcher with auto-reload
│   │   └── logger.rs          # Logging initialization
│   └── lua_engine.rs          # Lua engine integration (optional)
├── examples/
│   ├── config.example.toml    # Simple configuration example
│   └── config.example_full.toml # Complete configuration example
├── docs/                      # Documentation (Docusaurus site)
├── assets/                    # Static assets (logo, etc.)
├── Cargo.toml                 # Cargo manifest with dependencies
├── Cargo.lock                 # Cargo lock file
├── Makefile                   # Build and development commands
├── Dockerfile                 # Docker container configuration
├── README.md                  # Chinese documentation
├── README_en.md               # English documentation
└── AGENTS.md                  # This handover document
```

## Core Architecture

### Main Entry Point (`src/main.rs`)

The server starts by:

1. Parsing command-line arguments
2. Loading and validating configuration
3. Initializing logging
4. Starting initial servers based on config
5. Starting config file watcher for auto-reload
6. Handling shutdown signals

Key features:

- Uses Tokio async runtime
- Uses mimalloc as global allocator for better performance
- Configuration watcher with auto-reload on file change
- Graceful shutdown handling

### Configuration System (`src/config.rs`)

The configuration system uses TOML format and supports:

- Global settings: log level, log folder
- Multiple virtual hosts with separate configurations
- Per-host routes with:
  - Static file serving
  - Reverse proxying
  - Lua scripting
  - HTTP redirects
- SSL/TLS configuration
- Timeout settings

Key types:

- `Settings`: Root configuration struct
- `SettingHost`: Virtual host configuration
- `SettingRoute`: Individual route configuration (static/proxy/lua/redirect)

Validation:

- SSL certificate/key existence check
- Route location format validation
- Required fields validation
- File path validation

### HTTP Server (`src/http/mod.rs`)

The HTTP server uses Axum framework and axum-server for:

- Route registration and matching
- Virtual host support
- HTTPS with Rustls
- HTTP/2 support
- Timeout handling
- Compression
- Middleware chain (version header, custom headers, logging)

Key features:

- Route matching with optional trailing slash support
- Wildcard path handling for static files
- Per-route body size limits
- Host configuration stored in global `HOSTS` map (DashMap for concurrency)

### Configuration Watcher (`src/utils/config_watcher.rs`)

The config watcher uses the `notify` crate to monitor file changes with:

- Debounce mechanism to avoid frequent reloads
- Retry logic for failed config reads
- Re-watch logic for renamed/deleted files
- Async channel communication
- Graceful shutdown handling

Configuration options:

- `debounce_ms`: Debounce time for events (default: 500ms)
- `rewatch_delay_ms`: Delay after rename/delete events (default: 800ms)
- `max_retries`: Max retries for config read (default: 5)
- `retry_delay_ms`: Delay between retries (default: 100ms)

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

1. Standard library (std::\*)
2. External dependencies (alphabetical)
3. Internal modules (crate::_, super::_, self::\*)

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

````rust
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
````

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

- `src/main.rs`: Entry point and server lifecycle management
- `src/config.rs`: Configuration loading, validation, and struct definitions
- `src/http/mod.rs`: Axum server creation and route registration
- `src/utils/config_watcher.rs`: Config reloading on file change
- `src/lua_engine.rs`: Lua integration (optional feature)

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

## Dependencies Overview

| Dependency  | Version | Purpose                             |
| ----------- | ------- | ----------------------------------- |
| axum        | 0.8.8   | Web framework                       |
| axum-server | 0.8.0   | HTTP server implementation with TLS |
| hyper       | 1.8.1   | HTTP client/server core             |
| dashmap     | 6.1.0   | Concurrent hash map                 |
| notify      | 8.2.0   | File system watching                |
| mlua        | 0.11.5  | Lua integration (optional)          |
| mimalloc    | 0.1.48  | Memory allocator                    |
| tracing     | 0.1.44  | Logging system                      |
| toml        | 0.9.11  | TOML parsing                        |

## Configuration Example

```toml
log_level = "debug"
log_folder = "./logs"

[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false
timeout = 60
server_name = "localhost"

[[host.route]]
location = "/"
root = "./public"
index = ["index.html", "index.htm"]
auto_index = true

[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 30
max_body_size = 1048576

[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./cert.pem"
certificate_key = "./key.pem"
timeout = 30

[[host.route]]
location = "/"
root = "./ssl_public"
error_page = { status = 404, page = "/404.html" }
```

## Common Tasks

### Adding a New Feature

1. Create a new branch: `git checkout -b feat/your-feature`
2. Implement the feature
3. Add tests in the appropriate module
4. Run tests: `cargo test`
5. Run lint: `cargo clippy`
6. Format code: `cargo fmt`
7. Commit changes
8. Create PR

### Debugging

- Run in debug mode with logging: `cargo run -- --config path/to/config.toml`
- View detailed logs: Set `log_level = "trace"` in config
- Use `cargo test -- --nocapture` to see stdout in tests

### Performance Profiling

- Build with profiling: `cargo build --release --features flame`
- Use flamegraphs: `cargo flamegraph`
- Check memory usage with Valgrind: `valgrind --tool=massif target/release/candy`

## Troubleshooting

### Common Issues

1. **Configuration errors**: Check that all required fields are present and valid
2. **Port already in use**: Change port in config or kill existing process
3. **SSL certificate errors**: Verify certificate and key paths are correct
4. **Static file not found**: Check root path and file permissions
5. **Config reload not working**: Ensure watcher has read permissions on config file

## Handover Notes

Key areas to focus on:

- Configuration validation logic in `src/config.rs`
- Route matching and host selection in `src/http/mod.rs`
- Config watcher implementation in `src/utils/config_watcher.rs`
- Lua integration in `src/lua_engine.rs` and `src/http/lua/`
- Middleware chain in `src/middlewares/`

Current known limitations:

- No support for WebSocket proxying (planned)
- Limited Lua API (can be extended)
- No built-in caching mechanism

Future roadmap:

- WebSocket support
- Advanced routing rules
- Caching middleware
- Prometheus metrics
- Health check endpoints

