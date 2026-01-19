---
sidebar_position: 1
---

# Introduction

Candy is a lightweight, high-performance HTTP server written in Rust, designed to provide an easy-to-use deployment experience with powerful features. It supports static file serving, reverse proxying, Lua script handling, and HTTP redirection, making it suitable for quickly setting up web services.

## Key Features

- **Lightweight and Efficient**: Single binary with no dependencies, low resource consumption
- **High Performance**: Based on Tokio async runtime and Axum framework, supporting HTTP/2
- **Easy to Use**: Deploy with just one configuration file
- **SSL/TLS Support**: Built-in Rustls encryption, supporting HTTPS
- **Multiple Route Types**:
  - Static file hosting (with directory listing)
  - Reverse proxy (with timeout and body size limits)
  - Lua script handling (built-in Lua 5.4 engine)
  - HTTP redirection (supporting 301/302 status codes)
- **Virtual Hosts**: Support for port-based and domain-based virtual hosts
- **Compression**: Gzip, Deflate, and Brotli compression
- **Custom Error Pages**: Support for 404 and custom error pages

## Installation

### 1. Compile from Source

```bash
# Clone the repository
git clone https://github.com/DefectingCat/candy.git
cd candy

# Compile in release mode
cargo build --release

# Check the compiled executable
ls -la target/release/
```

### 2. Download Precompiled Binary

(To be added: Release page link and download instructions)

## Usage

Candy supports running as a single executable file:

```bash
❯ ./target/release/candy -h
Usage: candy [OPTIONS]

Options:
  -c, --config <FILE>  Sets a custom config file [default: ./config.toml]
  -h, --help           Print help
  -V, --version        Print version
```

### Quick Start

1. Create a configuration file `config.toml`:

```toml
log_level = "info"
log_folder = "./logs"

[[host]]
ip = "0.0.0.0"
port = 8080
server_name = "localhost"
timeout = 15

[[host.route]]
location = "/"
root = "./html"
index = ["index.html"]
auto_index = true
```

1. Create an `html` directory and add an `index.html` file:

```bash
mkdir html
echo "<h1>Hello from Candy!</h1>" > html/index.html
```

1. Start the server:

```bash
./target/release/candy
```

1. Access in your browser: `http://localhost:8080`

### Configuration File Location

The `-c` option specifies a custom configuration file path. If omitted, it defaults to `./config.toml` in the current directory.

## Quick Examples

### 1. Static File Server

```toml
[[host]]
ip = "0.0.0.0"
port = 8080
[[host.route]]
location = "/"
root = "./public"
index = ["index.html", "index.htm"]
auto_index = true
```

### 2. Reverse Proxy

```toml
[[host]]
ip = "0.0.0.0"
port = 8080
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 10
max_body_size = 1048576
```

### 3. Lua Script Handling

```toml
[[host]]
ip = "0.0.0.0"
port = 8080
[[host.route]]
location = "/hello"
lua_script = "./scripts/hello.lua"
```

`scripts/hello.lua`:

```lua
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Hello from Lua!")
```

## System Requirements

- **Operating System**: Linux, macOS, Windows
- **Memory**: At least 10MB of available memory
- **Disk Space**: At least 5MB of available space

## Supported Platforms

Candy can run on the following platforms:

- x86_64 (Intel/AMD)
- ARM (ARMv7, ARMv8)
- MIPS (partial support)

## Development and Contributing

Candy is an open-source project and contributions are welcome!

- **Repository**: [https://github.com/DefectingCat/candy](https://github.com/DefectingCat/candy)
- **License**: MIT License
