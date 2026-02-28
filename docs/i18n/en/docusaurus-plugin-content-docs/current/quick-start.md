---
sidebar_label: 快速入门
sidebar_position: 2
title: 快速入门
---

## 快速入门

本文档将帮助您快速上手使用 Candy 服务器。我们将介绍基本的安装、配置和使用方法。

## 系统要求

- **操作系统**：Linux、macOS、Windows、BSD 系统
- **CPU**：至少 1 核心（推荐 2 核心或更多）
- **内存**：至少 50MB 可用内存
- **磁盘空间**：至少 10MB 可用空间

## 安装方法

### 1. 从源代码编译

```bash
# 克隆仓库
git clone https://github.com/DefectingCat/candy.git
cd candy

# 编译发布版本
cargo build --release

# 查看编译好的可执行文件
ls -la target/release/
```

### 2. 使用预编译二进制文件

（待支持）

## 基本使用

### 1. 简单的静态文件服务器

创建一个简单的配置文件 `config.toml`：

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

### 2. 创建静态文件

创建 `html` 目录并添加 `index.html` 文件：

```bash
mkdir -p html
echo "<h1>Hello from Candy!</h1>" > html/index.html
```

### 3. 启动服务器

```bash
# 使用默认配置文件
candy

# 或者使用自定义配置文件
candy -c /path/to/config.toml
```

### 4. 访问服务器

在浏览器中访问 `http://localhost:8080`，您应该会看到 "Hello from Candy!"。

## 常用配置示例

### 1. 反向代理

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

### 2. 负载均衡

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

### 3. HTTPS 服务器

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

### 4. Lua 脚本处理

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

## 命令行选项

```bash
Usage: candy [OPTIONS]

Options:
  -c, --config <FILE>  Sets a custom config file [default: ./config.toml]
  -h, --help           Print help
  -V, --version        Print version
```

## 下一步

- 查看 [配置文件文档](./config/config-file) 了解详细配置选项
- 了解 [Lua 脚本编程](./config/lua) 功能
- 学习 [负载均衡](./config/load-balancing) 配置方法
- 掌握 [反向代理](./config/reverse-proxy) 高级用法
- 查看 [常见问题](./faq) 解决遇到的问题

## 资源

- [GitHub 仓库](https://github.com/DefectingCat/candy)
- [GitHub Issues](https://github.com/DefectingCat/candy/issues)
- [Contributing](https://github.com/DefectingCat/candy/blob/main/CONTRIBUTING.md)
- [CHANGELOG](https://github.com/DefectingCat/candy/blob/main/CHANGELOG.md)

## 许可证

Candy 遵循 MIT 许可证。详情请查看 LICENSE 文件。
