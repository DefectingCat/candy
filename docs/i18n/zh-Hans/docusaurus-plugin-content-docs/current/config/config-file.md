---
sidebar_label: 配置文件
sidebar_position: 1
title: 配置文件
---

## 配置文件

Candy 使用 TOML 格式的配置文件，默认名称为 `config.toml`。配置文件包含全局设置、上游服务器组和虚拟主机配置。

## 全局配置

### 日志配置

```toml
log_level = "info"  # 日志级别：trace/debug/info/warn/error（默认 info）
log_folder = "./logs"  # 日志文件夹路径（默认 ./logs）
```

#### 日志级别说明

- **trace**：最详细的日志，用于调试
- **debug**：详细的调试信息
- **info**：基本运行信息（默认）
- **warn**：警告信息
- **error**：错误信息

### 上游服务器组配置

```toml
[[upstream]]
name = "backend_servers"  # 服务器组名称（在路由中引用）
method = "weightedroundrobin"  # 负载均衡算法：roundrobin/weightedroundrobin/iphash（默认 weightedroundrobin）
server = [
    { server = "192.168.1.100:8080", weight = 3 },  # 权重 3
    { server = "192.168.1.101:8080", weight = 1 },  # 权重 1
    { server = "http://api1.example.com", weight = 2 },  # 支持 HTTP 协议前缀
    { server = "https://api2.example.com:443", weight = 1 }  # 支持 HTTPS
]
```

### 虚拟主机配置

顶层配置为虚拟主机 `host`，可以配置多个虚拟主机。

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"  # 服务器名称（域名），支持基于域名的路由
timeout = 15  # 连接超时（秒），默认 75 秒

# SSL/TLS 配置
ssl = true
# 自签名证书生成命令：
# sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout ./ssl/selfsigned.key -out ./ssl/selfsigned.crt
certificate = "./ssl/selfsigned.crt"
certificate_key = "./ssl/selfsigned.key"

# 主机级别的自定义响应头
[host.headers]
X-Powered-By = "Candy Server"
Cache-Control = "public, max-age=3600"
```

### 路由

每个虚拟主机下都可以配置多个路由。配置字段为 `route`。

每个路由支持四种配置：

- 静态文件托管
- 反向代理
- Lua 脚本
- HTTP 重定向

#### 静态文件托管

```toml
[[host.route]]
# 路由地址
location = "/"
# 静态文件根目录
root = "html"
# 当使用静态文件根目录时，使用下面的字段作为主页
index = ["index.html"]
# 列出目录
auto_index = true
```

#### 反向代理

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

#### Lua 脚本

```toml
[[host]]
ip = "0.0.0.0"
port = 8081
[[host.route]]
location = "/"
lua_script = "html/index.lua"
```

#### HTTP 重定向

```toml
[[host.route]]
location = "/old-path"
redirect_to = "http://example.com/new-path"  # 重定向目标 URL
redirect_code = 301  # 重定向状态码（301 永久，302 临时）
```

#### 错误页面

```toml
[[host.route]]
location = "/"
root = "html"

# 自定义错误页面（500 错误）
[host.route.error_page]
status = 500
page = "500.html"

# 自定义 404 页面
[host.route.not_found_page]
status = 404
page = "404.html"
```

#### 路由级自定义响应头

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"

# 路由级别的自定义响应头（覆盖主机配置）
[host.route.headers]
X-API-Version = "1.0"
```
