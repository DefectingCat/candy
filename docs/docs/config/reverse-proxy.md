---
sidebar_label: 反向代理
sidebar_position: 4
title: 反向代理
---

## 反向代理概述

Candy 支持将请求转发到后端服务器，提供反向代理功能。反向代理可以隐藏真实服务器地址、提供负载均衡、实现安全过滤和提高访问速度。

## 基本配置

### 1. 简单反向代理

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "api.example.com"

[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"  # 后端服务器地址
proxy_timeout = 10  # 连接超时（秒，默认：5）
max_body_size = 1048576  # 最大请求体大小（字节，默认：无限制）
```

### 2. 带路径重写的反向代理

```toml
[[host]]
ip = "0.0.0.0"
port = 80

[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000/v1"  # 将 /api 转发到 /v1
proxy_timeout = 10
```

### 3. HTTPS 反向代理

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"

[[host.route]]
location = "/"
proxy_pass = "https://api.example.com"  # 转发到 HTTPS 后端
proxy_timeout = 15
```

## 高级配置

### 1. 超时和连接配置

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 30  # 连接超时（秒）
max_body_size = 10485760  # 10MB 最大请求体
```

### 2. 自定义请求和响应头

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"

# 自定义响应头
[host.route.headers]
X-Proxy-By = "Candy"
X-API-Version = "1.0"
Cache-Control = "public, max-age=3600"
```

### 3. 错误处理

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"

# 自定义 500 错误页面
[host.route.error_page]
status = 500
page = "/500.html"

# 自定义 404 页面
[host.route.not_found_page]
status = 404
page = "/404.html"
```

## 正向代理

Candy 也支持作为正向代理使用，但这通常用于特殊场景：

```toml
[[host]]
ip = "0.0.0.0"
port = 8083
server_name = "proxy.example.com"

[[host.route]]
location = "/"
forward_proxy = true  # 启用正向代理
proxy_timeout = 30
max_body_size = 10485760  # 10MB 限制
```

## 反向代理与负载均衡结合

```toml
log_level = "info"
log_folder = "./logs"

# 定义上游服务器组
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
upstream = "backend"  # 引用上游服务器组
proxy_timeout = 10
max_body_size = 1048576
```

## 常见使用场景

### 1. API 网关

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "api.example.com"

# 用户服务
[[host.route]]
location = "/api/users"
proxy_pass = "http://localhost:3001"
proxy_timeout = 10
max_body_size = 1048576

# 订单服务
[[host.route]]
location = "/api/orders"
proxy_pass = "http://localhost:3002"
proxy_timeout = 15
max_body_size = 5242880

# 支付服务
[[host.route]]
location = "/api/payments"
upstream = "payment_servers"
proxy_timeout = 30
max_body_size = 2097152

# 静态资源
[[host.route]]
location = "/static"
root = "./static"
index = ["index.html"]
```

### 2. 应用服务器代理

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "app.example.com"

# 前端应用
[[host.route]]
location = "/"
root = "./frontend/build"
index = ["index.html"]

# API 代理
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 10
max_body_size = 1048576
```

### 3. 多环境部署

```toml
log_level = "info"
log_folder = "./logs"

# 开发环境
[[host]]
ip = "0.0.0.0"
port = 8080
server_name = "dev.example.com"

[[host.route]]
location = "/"
proxy_pass = "http://localhost:3000"

# 测试环境
[[host]]
ip = "0.0.0.0"
port = 8081
server_name = "test.example.com"

[[host.route]]
location = "/"
proxy_pass = "http://localhost:3001"

# 生产环境
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

## 性能优化

### 1. 超时设置

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 10  # 设置合理的超时时间
```

### 2. 请求体大小限制

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
max_body_size = 10485760  # 10MB 限制
```

### 3. 启用 HTTP/2

Candy 默认支持 HTTP/2，无需额外配置：

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"
# 自动支持 HTTP/2
```

## 安全考虑

### 1. 限制允许的请求方法

虽然 Candy 不直接支持，但可以通过 Lua 脚本实现：

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

-- 继续其他处理逻辑
candy.log("Valid method: " .. method)
```

### 2. 访问控制

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

-- 验证成功，继续代理请求
ctx:set_body("Welcome to admin panel")
```

## 故障排除

### 1. 连接超时

```toml
# 增加超时时间
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
proxy_timeout = 60  # 增加到 60 秒
```

### 2. 请求体过大

```toml
# 增加请求体大小限制
[[host.route]]
location = "/upload"
proxy_pass = "http://localhost:3000"
max_body_size = 52428800  # 50MB
```

### 3. 健康检查

```toml
# 使用多个后端服务器和负载均衡
[[upstream]]
name = "backend"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 1 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 1 }
]
```

## 最佳实践

1. **负载均衡**：使用上游服务器组提供高可用性
2. **监控**：定期监控后端服务器响应时间
3. **错误处理**：配置自定义错误页面提升用户体验
4. **安全**：使用 HTTPS 和适当的访问控制
5. **性能**：设置合理的超时和请求大小限制
6. **日志**：启用详细日志记录以进行故障排除
