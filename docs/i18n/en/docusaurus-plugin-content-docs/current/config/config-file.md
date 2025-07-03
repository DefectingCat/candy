---
sidebar_label: Config File
sidebar_position: 1
title: Config File
---

## Config File

Candy follows the config file to configure.

### Virtual Host

The top level configuration is the virtual host `host`, and can configure multiple virtual hosts.

```toml
[[host]]
ip = "0.0.0.0"
port = 80
# Connection timeout
timeout = 15
# Only read certificate and key when ssl = true, and enable SSL support
# ssl = true
# Self sign a certificate
# sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout ./html/selfsigned.key -out ./html/selfsigned.crt
certificate = "./html/selfsigned.crt"
certificate_key = "./html/selfsigned.key"
```

#### Custom HTTP Response Header

Each virtual host can configure custom response header.

TODO

### Route

Each virtual host can configure multiple routes. The configuration field is `route`.

Each route supports three configurations:

- Static file hosting
- Reverse proxy
- Lua script

#### Static File Hosting

```toml
[[host.route]]
# Route location
location = "/"
# Static file root
# or proxy_pass
# or redirect
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
