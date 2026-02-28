---
sidebar_position: 1
---

# Introduction

Candy is a lightweight, high-performance HTTP server written in Rust, designed to provide a simple and easy-to-use deployment experience with powerful features. It supports static file serving, reverse proxy, Lua script processing, and HTTP redirection, making it ideal for quickly setting up web services.

## Key Features

- **Lightweight and Efficient**: Single binary file with no dependencies, low resource consumption
- **High Performance**: Based on Tokio asynchronous runtime and Axum framework, supporting HTTP/2
- **Simple to Use**: Quick deployment with just one configuration file
- **SSL/TLS Support**: Built-in Rustls encryption, supporting HTTPS
- **Multi-route Support**:
  - Static file hosting (with directory listing support)
  - Reverse proxy (with timeout and body size limits)
  - Lua script processing (built-in Lua 5.4 engine)
  - HTTP redirection (supporting 301/302 status codes)
- **Virtual Hosts**: Support for port-based and domain-based virtual host configurations
- **Compression Support**: Gzip, Deflate, and Brotli compression
- **Custom Error Pages**: Support for 404 and custom error pages

## Installation

### 1. Build from Source

```bash
# Clone the repository
git clone https://github.com/DefectingCat/candy.git
cd candy

# Build release version
cargo build --release

# View the compiled executable
ls -la target/release/
```

### 2. Download Pre-built Binary

(TODO: Release page link and download instructions)

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

2. Create an `html` directory and add an `index.html` file:

```bash
mkdir html
echo "<h1>Hello from Candy!</h1>" > html/index.html
```

3. Start the server:

```bash
./target/release/candy
```

4. Visit in browser: `http://localhost:8080`

### Configuration File Location

The `-c` option can specify a custom configuration file path. If omitted, it defaults to `config.toml` in the current directory.

## Quick Examples

### 1. Static File Server

```toml
log_level = "info"
log_folder = "./logs"

[[host]]
ip = "0.0.0.0"
port = 8080
server_name = "localhost"

[[host.route]]
location = "/"
root = "./public"
index = ["index.html", "index.htm"]
auto_index = true
```

### 2. Reverse Proxy

```toml
log_level = "info"
log_folder = "./logs"

[[host]]
ip = "0.0.0.0"
port = 8080
server_name = "api.example.com"

[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 10
max_body_size = 1048576
```

### 3. Lua Script Processing

```toml
log_level = "info"
log_folder = "./logs"

[[host]]
ip = "0.0.0.0"
port = 8080
server_name = "lua.example.com"

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

### 4. Load Balancing

```toml
log_level = "info"
log_folder = "./logs"

# Upstream server group
[[upstream]]
name = "backend_servers"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 2 }
]

[[host]]
ip = "0.0.0.0"
port = 80
server_name = "loadbalance.example.com"

[[host.route]]
location = "/api"
upstream = "backend_servers"
proxy_timeout = 10
max_body_size = 1048576
```

### 5. HTTPS Server

```toml
log_level = "info"
log_folder = "./logs"

[[host]]
ip = "0.0.0.0"
port = 443
server_name = "secure.example.com"
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"

[[host.route]]
location = "/"
root = "./html"
```

### 6. Multiple Virtual Hosts

```toml
log_level = "info"
log_folder = "./logs"

# First virtual host (HTTP)
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"

[[host.route]]
location = "/"
root = "./html/example"

# Second virtual host (HTTPS)
[[host]]
ip = "0.0.0.0"
port = 443
server_name = "secure.example.com"
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"

[[host.route]]
location = "/"
root = "./html/secure"
```

## System Requirements

- **Operating Systems**: Linux, macOS, Windows
- **Memory**: At least 10MB available memory
- **Disk Space**: At least 5MB available space

## Supported Platforms

Candy can run on the following platforms:

- x86_64 (Intel/AMD)
- ARM (ARMv7, ARMv8)
- MIPS (partial support)

## Development and Contribution

Candy is an open-source project, contributions welcome!

- **Repository**: [https://github.com/DefectingCat/candy](https://github.com/DefectingCat/candy)
- **License**: MIT License