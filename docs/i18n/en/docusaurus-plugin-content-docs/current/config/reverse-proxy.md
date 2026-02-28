---
sidebar_label: Reverse Proxy
sidebar_position: 4
title: Reverse Proxy
---

## Reverse Proxy Overview

Candy supports forwarding requests to backend servers, providing reverse proxy functionality. Reverse proxy can hide real server addresses, provide load balancing, implement security filtering, and improve access speed.

## Basic Configuration

### 1. Simple Reverse Proxy

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "api.example.com"

[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"  # Backend server address
proxy_timeout = 10  # Connection timeout (seconds, default: 5)
max_body_size = 1048576  # Maximum request body size (bytes, default: unlimited)
```

### 2. Reverse Proxy with Path Rewrite

```toml
[[host]]
ip = "0.0.0.0"
port = 80

[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000/v1"  # Forward /api to /v1
proxy_timeout = 10
```

### 3. HTTPS Reverse Proxy

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"

[[host.route]]
location = "/"
proxy_pass = "https://api.example.com"  # Forward to HTTPS backend
proxy_timeout = 15
```

## Advanced Configuration

### 1. Timeout and Connection Configuration

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 30  # Connection timeout (seconds)
max_body_size = 10485760  # 10MB maximum request body
```

### 2. Custom Request and Response Headers

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"

# Custom response headers
[host.route.headers]
X-Proxy-By = "Candy"
X-API-Version = "1.0"
Cache-Control = "public, max-age=3600"
```

### 3. Error Handling

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"

# Custom 500 error page
[host.route.error_page]
status = 500
page = "/500.html"

# Custom 404 page
[host.route.not_found_page]
status = 404
page = "/404.html"
```

## Forward Proxy

Candy also supports forward proxy usage, though this is typically for special scenarios:

```toml
[[host]]
ip = "0.0.0.0"
port = 8083
server_name = "proxy.example.com"

[[host.route]]
location = "/"
forward_proxy = true  # Enable forward proxy
proxy_timeout = 30
max_body_size = 10485760  # 10MB limit
```

## Combining Reverse Proxy with Load Balancing

```toml
log_level = "info"
log_folder = "./logs"

# Define upstream server group
[[upstream]]
name = "backend"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 1 }
]

[[host]]
ip = "0.0.0.0"
port = 80
server_name = "api.example.com"

[[host.route]]
location = "/api"
upstream = "backend"  # Reference upstream server group
proxy_timeout = 10
max_body_size = 1048576
```

## Common Usage Scenarios

### 1. API Gateway

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "api.example.com"

# User service
[[host.route]]
location = "/api/users"
proxy_pass = "http://localhost:3001"
proxy_timeout = 10
max_body_size = 1048576

# Order service
[[host.route]]
location = "/api/orders"
proxy_pass = "http://localhost:3002"
proxy_timeout = 15
max_body_size = 5242880

# Payment service
[[host.route]]
location = "/api/payments"
upstream = "payment_servers"
proxy_timeout = 30
max_body_size = 2097152

# Static resources
[[host.route]]
location = "/static"
root = "./static"
index = ["index.html"]
```

### 2. Application Server Proxy

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "app.example.com"

# Frontend application
[[host.route]]
location = "/"
root = "./frontend/build"
index = ["index.html"]

# API proxy
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 10
max_body_size = 1048576
```

### 3. Multi-environment Deployment

```toml
log_level = "info"
log_folder = "./logs"

# Development environment
[[host]]
ip = "0.0.0.0"
port = 8080
server_name = "dev.example.com"

[[host.route]]
location = "/"
proxy_pass = "http://localhost:3000"

# Testing environment
[[host]]
ip = "0.0.0.0"
port = 8081
server_name = "test.example.com"

[[host.route]]
location = "/"
proxy_pass = "http://localhost:3001"

# Production environment
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"

[[host.route]]
location = "/"
upstream = "prod_servers"
proxy_timeout = 30
max_body_size = 10485760
```

## Performance Optimization

### 1. Timeout Settings

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 10  # Set reasonable timeout
```

### 2. Request Body Size Limit

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
max_body_size = 10485760  # 10MB limit
```

### 3. Enable HTTP/2

Candy supports HTTP/2 by default, no additional configuration required:

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"
# Automatically supports HTTP/2
```

## Security Considerations

### 1. Limit Allowed Request Methods

Although Candy doesn't support this directly, it can be achieved through Lua scripts:

```toml
[[host.route]]
location = "/api"
lua_script = "./scripts/validate_method.lua"
```

```lua
-- scripts/validate_method.lua
local allowed_methods = { "GET", "POST" }
local method = ctx:get_method()

if not allowed_methods[method] then
    ctx:set_status(405)
    ctx:set_header("Allow", "GET, POST")
    ctx:set_body("Method not allowed")
    return
end

-- Continue with other processing logic
candy.log("Valid method: " .. method)
```

### 2. Access Control

```toml
[[host.route]]
location = "/admin"
lua_script = "./scripts/admin_auth.lua"
```

```lua
-- scripts/admin_auth.lua
local auth_header = ctx:get_header("Authorization")

if not auth_header or auth_header ~= "Bearer secret_token" then
    ctx:set_status(401)
    ctx:set_header("WWW-Authenticate", "Bearer")
    ctx:set_body("Unauthorized")
    return
end

-- Authentication successful, continue proxy request
ctx:set_body("Welcome to admin panel")
```

## Troubleshooting

### 1. Connection Timeout

```toml
# Increase timeout time
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 60  # Increase to 60 seconds
```

### 2. Request Body Too Large

```toml
# Increase request body size limit
[[host.route]]
location = "/upload"
proxy_pass = "http://localhost:3000"
max_body_size = 52428800  # 50MB
```

### 3. Health Check

```toml
# Use multiple backend servers and load balancing
[[upstream]]
name = "backend"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 1 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 1 }
]
```

## Best Practices

1. **Load Balancing**: Use upstream server groups for high availability
2. **Monitoring**: Regularly monitor backend server response times
3. **Error Handling**: Configure custom error pages to enhance user experience
4. **Security**: Use HTTPS and appropriate access controls
5. **Performance**: Set reasonable timeouts and request size limits
6. **Logging**: Enable detailed logging for troubleshooting