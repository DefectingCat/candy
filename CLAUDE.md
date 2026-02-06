# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Candy** is a modern, lightweight web server written in Rust (version 0.2.4). Key features:

- Static file serving with directory listing support
- Reverse proxying with load balancing
- Lua scripting (optional feature)
- SSL/TLS encryption (HTTPS) with HTTP/2 support
- Auto-reload config on file change
- Multiple virtual hosts
- Forward proxy support
- HTTP redirect handling
- Custom error pages

## Build, Lint, and Test Commands

### Build

```bash
# Debug build
make build          # cargo build
make run            # cargo run

# Release build
make release        # cargo build --release

# Cross-compile for different platforms (using cross)
make linux-musl     # x86_64 Linux (Musl)
make aarch64-linux-musl  # ARM64 Linux (Musl)
make linux-gnu      # x86_64 Linux (GNU)
make windows-gnu    # Windows (GNU)
make freebsd        # FreeBSD
```

### Linting and Formatting

```bash
# Run linter (Clippy)
make lint           # cargo clippy

# Auto-format code (Rustfmt)
make format         # cargo fmt

# Auto-fix lint issues
make fix            # cargo fix + cargo fmt
```

### Testing

```bash
# Run all tests
make test           # cargo test

# Check compilation errors
make check          # cargo check
```

### Development Workflow

```bash
# Watch mode with live reload
make dev            # cargo watch -x run

# Clean builds
make clean          # cargo clean
make clean-release  # Remove release and debug targets
```

## High-Level Architecture

### Core Structure

```
src/
├── main.rs              # Entry point, server lifecycle management
├── config.rs            # Configuration loading, validation, and struct definitions
├── cli.rs               # Command-line argument parsing
├── consts.rs            # Constant definitions (version, build info, defaults)
├── error.rs             # Custom error types
├── http/
│   ├── mod.rs           # Axum server creation and route registration
│   ├── serve.rs         # Static file serving
│   ├── reverse_proxy.rs # Reverse proxy handling with load balancing
│   ├── forward_proxy.rs # Forward proxy support
│   ├── redirect.rs      # HTTP redirect handling
│   ├── lua.rs           # Lua script integration (optional)
│   └── error.rs         # HTTP-specific error types
├── utils/
│   ├── mod.rs           # Utility module
│   ├── config_watcher.rs # Config file watcher for auto-reload
│   ├── logging.rs       # Logging initialization
│   └── service.rs       # Service utilities
├── middlewares/         # Axum middleware implementations
└── lua_engine.rs        # Lua engine initialization (optional feature)
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
   - Graceful shutdown with configurable timeout

3. **Routing System**:
   - Routes defined per virtual host in config
   - Supports multiple route types: static files, reverse proxy, forward proxy, Lua, redirect
   - Route matching with location-based paths
   - Custom error pages and 404 handling

4. **Load Balancing**:
   - Upstream server groups defined in config
   - Supports Round Robin, Weighted Round Robin (default), and IP Hash algorithms
   - Session persistence with IP Hash

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

- `log_level`: Log verbosity (trace/debug/info/warn/error) - default: info
- `log_folder`: Directory for log files - default: ./logs
- `upstream`: Array of upstream server groups (for load balancing)
- `host`: Array of virtual hosts with listen addresses, SSL config, and routes

### Example Configuration

```toml
log_level = "info"
log_folder = "./logs"

[[upstream]]
name = "test_backend"
server = [
    { server = "192.168.1.100:8080" },
    { server = "192.168.1.101:8080", weight = 2 }
]
method = "weighted_round_robin"

[[host]]
ip = "0.0.0.0"
port = 8080
ssl = false
timeout = 30

[[host.route]]
location = "/"
root = "./html"
index = ["index.html", "index.htm"]
auto_index = true

[[host.route]]
location = "/api"
upstream = "test_backend"
proxy_timeout = 10
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
root = "./html/ssl"
error_page = { status = 404, page = "/404.html" }
```

## Key Modules and Their Responsibilities

### `src/main.rs`

- Entry point
- Command-line argument parsing
- Configuration loading and validation
- Logger initialization
- Server startup and shutdown management
- Config file watcher for auto-reload

### `src/config.rs`

- Defines configuration structs with Serde deserialization
- Configuration validation logic
- Upstream and host configuration parsing
- Default values for configuration fields
- Configuration validation tests

### `src/cli.rs`

- Command-line interface definition using Clap
- Arguments for config file path and other options

### `src/consts.rs`

- Version information (0.2.4)
- Build information (commit hash, compiler, OS)
- Default configuration values (log level, log folder, etc.)

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

### `src/http/forward_proxy.rs`

- Forward proxy implementation
- HTTP proxy for client requests

### `src/http/redirect.rs`

- HTTP redirect handling
- Route-based redirects to other URLs

### `src/utils/config_watcher.rs`

- Watches config file for changes using `notify` crate
- Triggers server restart with new config
- Handles errors during config reload

### `src/utils/logging.rs`

- Logging initialization using tracing crate
- File and console output configuration

### `src/lua_engine.rs` (Optional)

- Lua script engine initialization
- Lua API for request/response handling
- Integration with Axum request handlers

## Feature Flags

- `default`: Enables all features
- `lua`: Enables Lua scripting support (requires mlua dependency)

## Performance Optimizations

- Uses mimalloc allocator for better memory performance
- Compression middleware with multiple algorithms (gzip, deflate, brotli, zstd)
- HTTP/2 support for reduced latency
- Connection reuse and pipelining
- Optimized release profile with LTO and codegen units = 1
