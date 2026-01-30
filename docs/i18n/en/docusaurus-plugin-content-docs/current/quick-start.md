---
sidebar_label: Quick Start
sidebar_position: 2
title: Quick Start
---

## Quick Start

This document will help you quickly get started with the Candy server. We'll cover basic installation, configuration, and usage methods.

## System Requirements

- **Operating System**: Linux, macOS, Windows, BSD systems
- **CPU**: At least 1 core (2 or more recommended)
- **Memory**: At least 50MB available RAM
- **Disk Space**: At least 10MB available space

## Installation Methods

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

### 2. Using Precompiled Binaries

(Coming soon)

## Basic Usage

### 1. Simple Static File Server

Create a simple configuration file `config.toml`:

```toml
log_level = "info"
log_folder = "./logs"

[[host]]
ip = "0.0.0.0"
port = 8080
server_name = "localhost"

[[host.route]]
location = "/"
root = "./html"
index = ["index.html"]
auto_index = true
```

### 2. Create Static Files

Create an `html` directory and add an `index.html` file:

```bash
mkdir -p html
echo "<h1>Hello from Candy!</h1>" > html/index.html
```

### 3. Start the Server

```bash
# Use the default configuration file
candy

# Or use a custom configuration file
candy -c /path/to/config.toml
```

### 4. Access the Server

In your browser, visit `http://localhost:8080` - you should see "Hello from Candy!".

## Common Configuration Examples

### 1. Reverse Proxy

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

### 2. Load Balancing

```toml
log_level = "info"
log_folder = "./logs"

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

### 3. HTTPS Server

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

### 4. Lua Script Handling

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

```lua
-- scripts/hello.lua
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Hello from Lua!")
```

## Command-Line Options

```bash
Usage: candy [OPTIONS]

Options:
  -c, --config <FILE>  Sets a custom config file [default: ./config.toml]
  -h, --help           Print help
  -V, --version        Print version
```

## Next Steps

- Check the [Configuration File Documentation](./config/config-file) for detailed configuration options
- Learn about [Lua Scripting](./config/lua) capabilities
- Study [Load Balancing](./config/load-balancing) configuration methods
- Master [Reverse Proxy](./config/reverse-proxy) advanced usage
- View [Frequently Asked Questions](./faq) to solve common issues

## Resources

- [GitHub Repository](https://github.com/DefectingCat/candy)
- [GitHub Issues](https://github.com/DefectingCat/candy/issues)
- [Contributing](https://github.com/DefectingCat/candy/blob/main/CONTRIBUTING.md)
- [CHANGELOG](https://github.com/DefectingCat/candy/blob/main/CHANGELOG.md)

## License

Candy is released under the MIT License. For details, see the LICENSE file.
