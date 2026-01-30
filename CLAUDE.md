```
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
```

## Project Overview

**Candy** is a modern, lightweight web server written in Rust. Key features:

- Static file serving with directory listing support
- Reverse proxying with load balancing
- Lua scripting (optional feature)
- SSL/TLS encryption (HTTPS) with HTTP/2 support
- Auto-reload config on file change
- Multiple virtual hosts

## Docs

`docs/` 文件夹是使用 docusaurus 的文档项目。包管理器使用的是 `pnpm`

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

## High-Level Architecture

### Core Structure

```
src/
├── main.rs              # Entry point, server lifecycle management
├── config.rs            # Configuration loading, validation, and struct definitions
├── http/
│   ├── mod.rs           # Axum server creation and route registration
│   ├── serve.rs         # Static file serving
│   ├── reverse_proxy.rs # Reverse proxy handling with load balancing
│   ├── forward_proxy.rs # Forward proxy support
│   ├── redirect.rs      # HTTP redirect handling
│   └── lua.rs           # Lua script integration (optional)
├── utils/
│   ├── config_watcher.rs # Config file watcher for auto-reload
│   ├── logging.rs       # Logging initialization
│   └── service.rs       # Service utilities
├── middlewares/         # Axum middleware implementations
├── lua_engine.rs        # Lua engine initialization (optional feature)
└── error.rs             # Custom error types
```

### Key Design Patterns

1. **Configuration Management**:
   - Settings parsed from TOML config file
   - Validated before server startup
   - Supports auto-reload via file watcher
   - Stored in global static variables (`HOSTS`, `UPSTREAMS`) using DashMap for thread safety

2. **Server Architecture**:
   - Uses Axum web framework with Axum Server
   - Each virtual host runs in a separate tokio task
   - Supports HTTP and HTTPS (with optional HTTP/2)
   - Graceful shutdown with 30-second timeout

3. **Routing System**:
   - Routes defined per virtual host in config
   - Supports multiple route types: static files, reverse proxy, forward proxy, Lua, redirect
   - Route matching with wildcard support for paths with parameters
   - Handles both trailing-slash and non-trailing-slash path variants

4. **Load Balancing**:
   - Upstream server groups defined in config
   - Supports Round Robin, Weighted Round Robin, and IP Hash algorithms
   - Session persistence with IP Hash
   - Health checks not implemented (yet)

5. **Middleware Stack**:
   - Request logging
   - Response compression (gzip, deflate, brotli, zstd)
   - Request timeout
   - Custom headers
   - Server version header

## Configuration

### Main Configuration File

Default path: `config.toml`

Key sections:

- `log_level`: Log verbosity (trace/debug/info/warn/error)
- `log_folder`: Directory for log files
- `upstream`: Array of upstream server groups (for load balancing)
- `host`: Array of virtual hosts with listen addresses, SSL config, and routes

### Example Configuration

```toml
[server]
listen = "0.0.0.0:8080"
workers = 4
log_level = "info"

[virtual_hosts.default]
root = "./html"
index_files = ["index.html", "index.htm"]
directory_listing = true

[virtual_hosts.example]
server_name = "example.com"
root = "./examples/example.com"
index_files = ["index.html"]
```

## Key Modules and Their Responsibilities

### `src/main.rs`

- Entry point
- Configuration loading and validation
- Logger initialization
- Server startup and shutdown management
- Config file watcher for auto-reload

### `src/config.rs`

- Defines configuration structs with Serde deserialization
- Configuration validation logic
- Upstream and host configuration parsing
- Default values for configuration fields

### `src/http/mod.rs`

- Server creation (`make_server`)
- Route registration based on config
- Host and upstream configuration storage
- Server lifecycle management (start/stop/shutdown)

### `src/http/serve.rs`

- Static file serving with directory listing
- MIME type detection
- Error handling for missing files/directories

### `src/http/reverse_proxy.rs`

- Reverse proxy implementation
- Load balancing across upstream servers
- Request/response headers handling
- Timeout management

### `src/utils/config_watcher.rs`

- Watches config file for changes using `notify` crate
- Triggers server restart with new config
- Handles errors during config reload

### `src/lua_engine.rs` (Optional)

- Lua script engine initialization
- Lua API for request/response handling
- Integration with Axum request handlers

## Feature Flags

- `default`: Enables all features
- `lua`: Enables Lua scripting support (requires mlua dependency)

## Performance Optimizations

- Uses mimalloc allocator for better memory performance
- Compression middleware with multiple algorithms
- HTTP/2 support for reduced latency
- Connection reuse and pipelining
