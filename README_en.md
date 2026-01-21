# Candy

<img src="./assets/candy-transparent.png" width="200px">

A modern, lightweight web server written in Rust.

[![dependency status](https://deps.rs/repo/github/DefectingCat/candy/status.svg)](https://deps.rs/repo/github/DefectingCat/candy)
![](https://git.rua.plus/xfy/candy/badges/main/pipeline.svg)
![](https://git.rua.plus/xfy/candy/-/badges/release.svg)

## Features

- **Static file serving** - Serve static files with directory listing support
- **Reverse proxying** - Proxy requests to backend servers
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

## Build Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Run
cargo run -- --config config.toml

# Run tests
cargo test
```

## Documentation

- [Configuration Guide](docs/) - Detailed configuration options
- [Examples](examples/) - Usage examples
- [CHANGELOG](CHANGELOG.md) - Release history
- [TODO](TODO.md) - Planned features

## License

[MIT](LICENSE)
