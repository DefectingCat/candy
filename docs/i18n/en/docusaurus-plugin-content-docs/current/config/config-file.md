---
sidebar_label: Config File
sidebar_position: 1
title: Config File
---

## Config File

Candy follows the config file to configure. The configuration file format is TOML.

## Global Configuration

```toml
log_level = "info"  # Log level: trace/debug/info/warn/error (default info)
log_folder = "./logs"  # Log folder path (default ./logs)
```

### Virtual Host

The top level configuration is the virtual host `host`, and can configure multiple virtual hosts.

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"  # Server name (domain name), supports domain-based routing
timeout = 15  # Connection timeout

# Only read certificate and key when ssl = true, and enable SSL support
# ssl = true
# Self sign a certificate
# sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout ./html/selfsigned.key -out ./html/selfsigned.crt
certificate = "./html/selfsigned.crt"
certificate_key = "./html/selfsigned.key"

# Host-level custom response headers
[host.headers]
X-Powered-By = "candy"
```

### Route

Each virtual host can configure multiple routes. The configuration field is `route`.

Each route supports four configurations:

- Static file hosting
- Reverse proxy
- Lua script
- HTTP redirect

#### Static File Hosting

```toml
[[host.route]]
# Route location
location = "/"
# Static file root
root = "html"
# Only use for root field
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

#### HTTP Redirect

```toml
[[host.route]]
location = "/old-path"
redirect_to = "http://example.com/new-path"  # Redirect target URL
redirect_code = 301  # Redirect status code (301 permanent, 302 temporary)
```

#### Error Page

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

#### Route-level Custom Response Header

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"

# Route-level custom response headers (overrides host configuration)
[host.route.headers]
X-API-Version = "1.0"
```
