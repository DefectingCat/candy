# Candy

<img src="./assets/candy-transparent.png" width="200px">

A modern, lightweight web server written in Rust.

[![dependency status](https://deps.rs/repo/github/DefectingCat/candy/status.svg)](https://deps.rs/repo/github/DefectingCat/candy)
![](https://git.rua.plus/xfy/candy/badges/main/pipeline.svg)
![](https://git.rua.plus/xfy/candy/-/badges/release.svg)

## Features

- **Static file serving** - Serve static files with directory listing support
- **Reverse proxying** - Proxy requests to backend servers with round-robin load balancing
- **Lua scripting** - Extend functionality with Lua scripts (optional feature)
- **SSL/TLS encryption** - Secure connections with HTTPS
- **HTTP/2 support** - Modern protocol support for faster performance
- **Configuration reload** - Auto-reload config on file change
- **Multiple virtual hosts** - Host multiple websites on a single server
- **Single binary** - Easy to deploy with no dependencies

## Quick Start

### Installation

```bash
# Build from source (requires Rust)
git clone https://github.com/DefectingCat/candy.git
cd candy
cargo build --release
```

### Configuration

Copy and customize the example config:
```bash
cp config.example.toml config.toml
# Edit config.toml to your needs
```

### Run

```bash
# Run with default config (config.toml)
cargo run --release

# Or run directly
./target/release/candy --config path/to/config.toml
```

## Using Makefile

The project provides a Makefile to simplify common operations:

```bash
# Build (debug)
make build

# Build (release)
make release

# Run (debug mode)
make run

# Run with arguments
make run ARGS="--config path/to/config.toml"

# Development mode (auto-reload)
make dev

# Run all tests
make test

# Code formatting
make format

# Code linting
make lint

# Fix common lint issues
make fix

# Check compilation
make check
```

## Configuration Example

A simple configuration example:

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

## Documentation

- [Configuration Guide](docs/) - Detailed configuration options
- [Examples](examples/) - Usage examples for various scenarios
- [CHANGELOG](CHANGELOG.md) - Release history and changes
- [TODO](TODO.md) - Planned features

## License

[MIT](LICENSE)
