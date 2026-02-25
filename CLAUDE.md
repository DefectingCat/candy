```
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
```

# Candy 项目开发指南

## 项目概述

Candy 是一个用 Rust 编写的现代、轻量级 Web 服务器（版本 0.2.5）。它提供了静态文件服务、反向代理、负载均衡、Lua 脚本支持等功能，是一个高性能且易于配置的服务器解决方案。

## 核心功能

- 静态文件服务，支持目录列表
- 反向代理，支持负载均衡（轮询、加权轮询、IP 哈希、最少连接）
- Lua 脚本支持（可选功能）
- SSL/TLS 加密（HTTPS），支持 HTTP/2
- 配置文件变更自动重载（带防抖机制）
- 多虚拟主机支持
- 正向代理支持
- HTTP 重定向处理
- 自定义错误页面
- 健康检查功能（主动和被动）
- 详细的调试日志和监控

## 技术栈

- **Web 框架**: Axum（异步、高性能）
- **服务器**: Axum Server（HTTP/1.1 + HTTP/2）
- **异步运行时**: Tokio
- **日志**: Tracing（含文件日志和控制台输出）
- **配置**: Serde + TOML（带验证和自动重载）
- **压缩**: Axum 压缩中间件（gzip、deflate、brotli、zstd）
- **Lua 支持**: Mlua（可选，Lua 5.4）
- **配置监听**: Notify 库（带防抖机制）
- **数据结构**: DashMap（并发安全哈希表）
- **HTTP 客户端**: Reqwest（支持 HTTP/2 和多种压缩格式）

## 常用命令

### 构建和运行

```bash
# 调试构建
make build
make run

# 发布构建
make release

# 开发模式（自动重载）
make dev            # 使用 cargo watch 自动重载

# 运行服务器
candy               # 使用默认配置文件（config.toml）
candy -c /path/to/config.toml  # 使用指定配置文件
candy --help        # 查看帮助信息
```

### 代码质量和测试

```bash
# 格式化代码
make format         # 使用 rustfmt 格式化
cargo fmt           # 直接使用 cargo

# 运行 Clippy 检查
make lint           # 运行 Clippy 检查
cargo clippy        # 直接使用 cargo

# 自动修复 lint 问题并格式化
make fix

# 运行测试
make test           # 运行所有测试（单线程）
cargo test          # 直接使用 cargo
cargo test -- --test-threads=1  # 单线程测试（避免资源竞争）

# 检查编译错误
make check
cargo check

# 运行单个测试
cargo test <test_name> -- --test-threads=1
```

## 项目架构

### 核心模块

```
/Users/xfy/Developer/candy/src/
├── main.rs              # 入口点，服务器生命周期管理
├── cli.rs               # 命令行参数解析
├── config.rs            # 配置加载、验证和结构体定义
├── consts.rs            # 常量定义
├── error.rs             # 自定义错误类型
├── http/                # HTTP 相关模块
│   ├── mod.rs           # 服务器创建和路由注册
│   ├── serve.rs         # 静态文件服务
│   ├── reverse_proxy.rs # 反向代理实现（含负载均衡）
│   ├── forward_proxy.rs # 正向代理实现
│   ├── redirect.rs      # 重定向处理
│   └── lua.rs           # Lua 脚本集成（可选）
├── lua_engine.rs        # Lua 引擎初始化（可选特性）
├── middlewares/         # Axum 中间件实现
└── utils/               # 工具模块
    ├── mod.rs           # 工具模块入口
    ├── config_watcher.rs # 配置文件监听（自动重载）
    ├── logging.rs       # 日志初始化
    └── service.rs       # 服务工具
```

### 关键架构特点

1. **异步架构**: 基于 Tokio 异步运行时和 Axum 框架
2. **并发安全**: 使用 DashMap 实现高性能并发数据结构
3. **模块化设计**: 清晰的模块划分，各功能独立实现
4. **配置驱动**: 完整的配置验证和自动重载机制
5. **可扩展性**: 支持可选特性（如 Lua 脚本）通过 Cargo features 实现

## 配置文件

### 主配置文件结构

默认配置文件路径：`config.toml`

```toml
log_level = "info"
log_folder = "./logs"

# 上游服务器组（用于负载均衡）
[[upstream]]
name = "test_backend"
server = [
    { server = "192.168.1.100:8080" },
    { server = "192.168.1.101:8080", weight = 2 }
]
method = "weighted_round_robin"  # 负载均衡方法：round_robin/weighted_round_robin/ip_hash/least_conn

# 虚拟主机配置
[[host]]
ip = "0.0.0.0"
port = 8080
ssl = false
timeout = 30

# 路由配置
[[host.route]]
location = "/"
root = "./html"
index = ["index.html", "index.htm"]
auto_index = true

[[host.route]]
location = "/api"
upstream = "test_backend"
proxy_timeout = 10
max_body_size = 1048576
```

### 负载均衡算法

- **RoundRobin**: 简单轮询（默认）
- **WeightedRoundRobin**: 加权轮询，支持服务器权重配置
- **IpHash**: IP 哈希算法，实现会话保持
- **LeastConn**: 最少连接数算法，动态分配请求到连接数最少的服务器

## 开发流程

### 调试

- 使用 `RUST_BACKTRACE=full` 环境变量获取完整堆栈跟踪
- 使用 `log_level = "trace"` 在配置文件中启用详细日志
- 使用 `cargo run -- --help` 查看命令行选项

### 添加新功能

1. 确定功能所属模块或创建新模块
2. 实现功能逻辑
3. 更新配置结构（如有需要）
4. 更新路由注册（src/http/mod.rs）
5. 编写测试
6. 运行 `make lint` 和 `make test`
7. 提交代码

### 修改配置

- 编辑 `config.toml` 文件
- 服务器会自动检测变化并重启
- 确保配置结构与 `src/config.rs` 中的定义匹配

## 监控与维护

### 日志

- 默认日志目录：`./logs/`
- 日志文件格式：`candy-[日期].log`
- 使用 `tail -f logs/candy-[日期].log` 实时查看日志

### 常见问题

1. **配置验证失败**：检查配置文件格式和值是否符合要求
2. **端口占用**：使用 `lsof -i :端口号` 查找占用进程
3. **SSL 证书问题**：确保证书和密钥文件路径正确，权限合适
4. **性能问题**：检查上游服务器响应时间，优化负载均衡配置

## 性能优化

### 已实现的优化

- 使用 MiMalloc 内存分配器替代默认分配器
- 启用 HTTP/2 支持
- 实现请求和响应压缩
- 优化连接池管理
- 使用并发安全数据结构

### 构建优化

发布构建配置（Cargo.toml）：
- `opt-level = 3`：最高优化级别
- `strip = true`：移除调试符号
- `lto = true`：启用链接时优化
- `codegen-units = 1`：允许更好的优化

## 测试覆盖

项目包含全面的单元测试，覆盖：
- 配置验证
- 服务器启动和关闭
- 静态文件服务
- 反向代理和负载均衡
- 健康检查
- 配置文件监听和重载
- 路由匹配

运行所有测试：
```bash
make test
```
