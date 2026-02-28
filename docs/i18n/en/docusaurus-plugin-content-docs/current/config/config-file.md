---
sidebar_label: Configuration File
sidebar_position: 1
title: Configuration File
---

## Configuration File

Candy uses TOML format configuration files, with the default name `config.toml`. The configuration file contains global settings, upstream server groups, and virtual host configurations.

## Global Configuration

### Log Configuration

```toml
log_level = "info"  # Log level: trace/debug/info/warn/error (default info)
log_folder = "./logs"  # Log folder path (default ./logs)
```

#### Log Level Description

- **trace**: Most detailed logging, for debugging
- **debug**: Detailed debug information
- **info**: Basic runtime information (default)
- **warn**: Warning information
- **error**: Error information

### Upstream Server Group Configuration

```toml
[[upstream]]
name = "backend_servers"  # Server group name (referenced in routes)
method = "weightedroundrobin"  # Load balancing algorithm: roundrobin/weightedroundrobin/iphash (default weightedroundrobin)
server = [
    { server = "192.168.1.100:8080", weight = 3 },  # Weight 3
    { server = "192.168.1.101:8080", weight = 1 },  # Weight 1
    { server = "http://api1.example.com", weight = 2 },  # Supports HTTP protocol prefix
    { server = "https://api2.example.com:443", weight = 1 }  # Supports HTTPS
]
```

### Virtual Host Configuration

Top-level configuration is the virtual host `host`, and multiple virtual hosts can be configured.

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"  # Server name (domain), supports domain-based routing
timeout = 15  # Connection timeout (seconds), default 75 seconds

# SSL/TLS Configuration
ssl = true
# Self-signed certificate generation command:
# sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout ./ssl/selfsigned.key -out ./ssl/selfsigned.crt
certificate = "./ssl/selfsigned.crt"
certificate_key = "./ssl/selfsigned.key"

# Host-level custom response headers
[host.headers]
X-Powered-By = "Candy Server"
Cache-Control = "public, max-age=3600"
```

### Routes

Multiple routes can be configured under each virtual host. Configuration field is `route`.

Each route supports four configurations:

- Static file hosting
- Reverse proxy
- Lua script
- HTTP redirection

#### Static File Hosting

```toml
[[host.route]]
# Route address
location = "/"
# Static file root directory
root = "html"
# When using static file root directory, use the following fields as homepage
index = ["index.html"]
# List directory
auto_index = true
```

#### Reverse Proxy

```toml
[[host]]
ip = "0.0.0.0"
port = 8080
[[host.route]]
location = "/"
proxy_pass = "http://localhost:3000/"
# Timeout for connect to upstream
proxy_timeout = 10
# Client request max body size (bytes)
max_body_size = 2048
```

#### Lua Script

```toml
[[host]]
ip = "0.0.0.0"
port = 8081
[[host.route]]
location = "/"
lua_script = "html/index.lua"
```

#### HTTP Redirection

```toml
[[host.route]]
location = "/old-path"
redirect_to = "http://example.com/new-path"  # Redirect target URL
redirect_code = 301  # Redirect status code (301 permanent, 302 temporary)
```

#### Error Pages

```toml
[[host.route]]
location = "/"
root = "html"

# Custom error page (500 error)
[host.route.error_page]
status = 500
page = "500.html"

# Custom 404 page
[host.route.not_found_page]
status = 404
page = "404.html"
```

#### Route-level Custom Response Headers

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"

# Route-level custom response headers (overrides host configuration)
[host.route.headers]
X-API-Version = "1.0"
```
