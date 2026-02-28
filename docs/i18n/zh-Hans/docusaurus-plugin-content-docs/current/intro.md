---
sidebar_position: 1
---

# 介绍

Candy 是一个用 Rust 语言编写的轻量级、高性能 HTTP 服务器，旨在提供简单易用的部署体验和强大的功能特性。它支持静态文件服务、反向代理、Lua 脚本处理和 HTTP 重定向等功能，适合快速搭建 Web 服务。

## 主要特性

- **轻量高效**：单二进制文件，无依赖，资源消耗低
- **高性能**：基于 Tokio 异步运行时和 Axum 框架，支持 HTTP/2
- **简单易用**：只需一个配置文件即可快速部署
- **SSL/TLS 支持**：内置 Rustls 加密，支持 HTTPS
- **多路由支持**：
  - 静态文件托管（支持目录列表）
  - 反向代理（支持超时和 body 大小限制）
  - Lua 脚本处理（内置 Lua 5.4 引擎）
  - HTTP 重定向（支持 301/302 状态码）
- **虚拟主机**：支持基于端口和域名的虚拟主机配置
- **压缩支持**：Gzip、Deflate、Brotli 压缩
- **自定义错误页面**：支持 404 和自定义错误页面

## 安装

### 1. 从源码编译

```bash
# 克隆仓库
git clone https://github.com/DefectingCat/candy.git
cd candy

# 编译发布版本
cargo build --release

# 查看编译好的可执行文件
ls -la target/release/
```

### 2. 下载预编译二进制文件

（待补充：发布页面链接和下载说明）

## 使用

Candy 支持单个可执行文件运行：

```bash
❯ ./target/release/candy -h
Usage: candy [OPTIONS]

Options:
  -c, --config <FILE>  Sets a custom config file [default: ./config.toml]
  -h, --help           Print help
  -V, --version        Print version
```

### 快速启动

1. 创建配置文件 `config.toml`：

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

1. 创建 `html` 目录并添加 `index.html` 文件：

```bash
mkdir html
echo "<h1>Hello from Candy!</h1>" > html/index.html
```

1. 启动服务器：

```bash
./target/release/candy
```

1. 在浏览器中访问：`http://localhost:8080`

### 配置文件位置

`-c` 选项可以指定自定义配置文件路径，省略时默认使用当前目录下的 `config.toml` 文件。

## 快速示例

### 1. 静态文件服务器

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

### 2. 反向代理

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

### 3. Lua 脚本处理

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

`scripts/hello.lua`：

```lua
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Hello from Lua!")
```

### 4. 负载均衡

```toml
log_level = "info"
log_folder = "./logs"

# 上游服务器组
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

### 5. HTTPS 服务器

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

### 6. 多虚拟主机

```toml
log_level = "info"
log_folder = "./logs"

# 第一个虚拟主机（HTTP）
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"

[[host.route]]
location = "/"
root = "./html/example"

# 第二个虚拟主机（HTTPS）
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

## 系统要求

- **操作系统**：Linux、macOS、Windows
- **内存**：至少 10MB 可用内存
- **磁盘空间**：至少 5MB 可用空间

## 支持的平台

Candy 可以在以下平台上运行：

- x86_64（Intel/AMD）
- ARM（ARMv7、ARMv8）
- MIPS（部分支持）

## 开发与贡献

Candy 是一个开源项目，欢迎贡献！

- **仓库地址**：[https://github.com/DefectingCat/candy](https://github.com/DefectingCat/candy)
- **许可证**：MIT License
