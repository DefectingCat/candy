---
sidebar_label: 配置文件
sidebar_position: 1
title: 配置文件
---

## 配置文件

Candy 遵循配置文件进行配置。配置文件的格式为 TOML。

### 虚拟主机

顶层配置为虚拟主机 `host`，可以配置多个虚拟主机。

```toml
[[host]]
ip = "0.0.0.0"
port = 80
# Connection timeout
timeout = 15
# 只用当 ssl = true 时，才会读取证书和密钥，并开启 SSL 支持
# ssl = true
# Self sign a certificate
# sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout ./html/selfsigned.key -out ./html/selfsigned.crt
certificate = "./html/selfsigned.crt"
certificate_key = "./html/selfsigned.key"
```

#### 自定义 HTTP 相应头

每个虚拟主机都可以配置自定义相应头

TODO

### 路由

每个虚拟主机下都可以配置多个路由。配置字段为 `route`。

每个路由支持三种配置：

- 静态文件托管
- 反向代理
- Lua 脚本

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
