---
sidebar_label: 负载均衡
sidebar_position: 3
title: 负载均衡
---

## 负载均衡概述

Candy 支持多种负载均衡算法，允许您将请求分发到多个后端服务器，提高应用程序的可用性和性能。负载均衡功能通过 `upstream` 配置实现。

## 配置方法

### 1. 定义上游服务器组

在配置文件中定义一个或多个上游服务器组，每个组包含多个服务器地址和权重：

```toml
# 定义上游服务器组
[[upstream]]
name = "backend_servers"  # 服务器组名称（在路由中引用）
method = "weightedroundrobin"  # 负载均衡算法（默认：weightedroundrobin）
server = [
    { server = "192.168.1.100:8080", weight = 3 },  # 权重 3
    { server = "192.168.1.101:8080", weight = 1 },  # 权重 1
    { server = "http://api1.example.com", weight = 2 },  # 支持 HTTP 协议前缀
    { server = "https://api2.example.com:443", weight = 1 }  # 支持 HTTPS
]

# 第二个上游服务器组（IP 哈希算法）
[[upstream]]
name = "session_servers"
method = "iphash"  # IP 哈希算法
server = [
    { server = "192.168.1.102:8080", weight = 1 },
    { server = "192.168.1.103:8080", weight = 1 },
    { server = "192.168.1.104:8080", weight = 1 }
]
```

### 2. 在路由中使用上游服务器组

在虚拟主机路由配置中引用定义好的上游服务器组：

```toml
[[host]]
ip = "0.0.0.0"
port = 8084
server_name = "loadbalance.example.com"

[[host.route]]
location = "/api"
upstream = "backend_servers"  # 引用上游服务器组名称
proxy_timeout = 30  # 代理超时（秒）
max_body_size = 1048576  # 最大请求体大小（字节）
```

## 负载均衡算法

Candy 支持以下三种负载均衡算法：

### 1. 加权轮询（Weighted Round Robin）- 默认

```toml
method = "weightedroundrobin"
```

- 按权重比例分发请求
- 权重值越大，分配到的请求越多
- 适合服务器性能差异较大的场景
- **示例配置：**

```toml
[[upstream]]
name = "weighted_servers"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },  # 处理 3/7 的请求
    { server = "192.168.1.101:8080", weight = 2 },  # 处理 2/7 的请求
    { server = "192.168.1.102:8080", weight = 1 },  # 处理 1/7 的请求
    { server = "192.168.1.103:8080", weight = 1 }   # 处理 1/7 的请求
]
```

### 2. 轮询（Round Robin）

```toml
method = "roundrobin"
```

- 按顺序依次分发请求到每个服务器
- 所有服务器权重相等
- 适合服务器性能相似的场景

### 3. IP 哈希（IP Hash）

```toml
method = "iphash"
```

- 基于客户端 IP 地址的哈希值选择服务器
- 相同 IP 的请求会始终路由到同一服务器
- 适合需要会话保持的应用场景
- **注意：** 如果服务器列表发生变化，会话可能会中断

## 服务器权重

权重参数用于加权轮询算法，范围为 1-255。权重值越高，服务器接收的请求比例越大。

```toml
server = [
    { server = "server1:8080", weight = 5 },  # 处理 50% 的请求
    { server = "server2:8080", weight = 3 },  # 处理 30% 的请求
    { server = "server3:8080", weight = 2 }   # 处理 20% 的请求
]
```

## 配置示例

### 1. 基本负载均衡

```toml
log_level = "info"
log_folder = "./logs"

# 定义上游服务器组
[[upstream]]
name = "backend"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 2 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 1 }
]

# 虚拟主机配置
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "api.example.com"

[[host.route]]
location = "/"
upstream = "backend"
proxy_timeout = 10
max_body_size = 1048576
```

### 2. 会话保持（IP Hash）

```toml
log_level = "info"
log_folder = "./logs"

# 定义 IP 哈希服务器组
[[upstream]]
name = "session_aware"
method = "iphash"
server = [
    { server = "192.168.1.100:8080", weight = 1 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 1 }
]

[[host]]
ip = "0.0.0.0"
port = 80
server_name = "app.example.com"

[[host.route]]
location = "/app"
upstream = "session_aware"
proxy_timeout = 30
max_body_size = 10485760
```

### 3. 多种负载均衡策略组合

```toml
log_level = "info"
log_folder = "./logs"

# API 服务器组（加权轮询）
[[upstream]]
name = "api_servers"
method = "weightedroundrobin"
server = [
    { server = "api1.example.com:8080", weight = 3 },
    { server = "api2.example.com:8080", weight = 2 },
    { server = "api3.example.com:8080", weight = 1 }
]

# 静态资源服务器组（轮询）
[[upstream]]
name = "static_servers"
method = "roundrobin"
server = [
    { server = "static1.example.com:80", weight = 1 },
    { server = "static2.example.com:80", weight = 1 }
]

[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"

# API 路由
[[host.route]]
location = "/api"
upstream = "api_servers"
proxy_timeout = 10
max_body_size = 1048576

# 静态资源路由
[[host.route]]
location = "/static"
upstream = "static_servers"
proxy_timeout = 30
max_body_size = 10485760
```

## 健康检查（待实现）

当前版本的 Candy 不支持主动健康检查功能。如果需要实现服务器健康检查，可以考虑：

1. 使用外部健康检查工具（如 Prometheus + Alertmanager）
2. 配置服务器级别的超时和重试机制
3. 定期监控服务器响应状态

## 最佳实践

1. **服务器监控**：定期检查服务器状态和响应时间
2. **权重配置**：根据服务器性能合理分配权重
3. **会话管理**：需要会话保持时使用 IP 哈希算法
4. **服务器数量**：至少配置 2 台服务器以保证可用性
5. **超时设置**：合理设置代理超时时间，避免长时间等待
6. **最大请求体**：根据实际业务需求设置最大请求体大小

## 限制

- 不支持主动健康检查
- 不支持服务器动态上下线
- 不支持连接池配置
- 不支持流量镜像功能
