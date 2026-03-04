```
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
```

# Candy 项目开发指南

Candy 是一个用 Rust 编写的现代、轻量级 Web 服务器。提供静态文件服务、反向代理、负载均衡、Lua 脚本支持等功能。

## 规范

- candy 的 lua 全局变量为 cd（candy 的缩写）
- 每次新增/修改某个功能时都必须要保证其对应的测试通过
- 每个功能都必须有单元测试/集成测试
- 多个任务使用 TODO list 来规划
- 代码必须优雅且高性能
  - 可以先不优雅的去实现功能
  - 然后再优化
- 函数的文档必须是 rust doc 格式且详细

函数文档示例：

```rust
/// 构建压缩层
///
/// 根据路由配置和全局配置构建压缩层。路由级别配置优先于全局配置。
///
/// # 参数
/// * `route` - 路由配置
/// * `global` - 全局压缩配置
///
/// # 返回值
/// 返回构建好的压缩层
fn build_compression_layer(route: &SettingRoute, global: &CompressionConfig) -> CompressionLayer {
    // ..
}
```

## 常用命令

```bash
# 构建
make build          # 调试构建
make release        # 发布构建

# 运行
make run            # 运行调试版本
make run ARGS="--config path/to/config.toml"  # 带参数运行

# 代码质量
make check          # 检查编译错误
make format         # 格式化代码
make lint           # 运行 Clippy 检查
make fix            # 自动修复 lint 问题并格式化

# 测试
make test           # 运行所有测试
cargo test
cargo test <test_name># 运行单个测试
```

## 架构概览

```
src/
├── main.rs              # 入口点：配置加载、服务器启动、配置热重载
├── config.rs            # 配置结构体定义和验证
├── cli.rs               # 命令行参数解析
├── http/
│   ├── mod.rs           # 服务器创建、路由注册、全局状态（HOSTS/UPSTREAMS）
│   ├── serve.rs         # 静态文件服务
│   ├── reverse_proxy.rs # 反向代理 + 负载均衡 + 健康检查
│   ├── forward_proxy.rs # 正向代理
│   └── lua/             # Lua 脚本处理（可选特性）
├── lua_engine/          # Lua 引擎初始化和共享字典（可选特性）
├── middlewares/         # Axum 中间件
└── utils/
    ├── config_watcher.rs # 配置文件监听（自动重载）
    └── logging.rs        # 日志初始化
```

### 关键数据结构

- **HOSTS**: `DashMap<u16, DashMap<Option<String>, SettingHost>>` - 按端口和域名存储主机配置
- **UPSTREAMS**: `DashMap<String, Upstream>` - 上游服务器组配置
- **路由优先级**: redirect_to > lua_script > proxy_pass/upstream > forward_proxy > root

### 请求处理流程

1. 配置文件解析 → `Settings::new()` 验证并构建配置
2. 服务器启动 → `start_initial_servers()` 为每个 host 创建 Axum 实例
3. 路由注册 → `make_server()` 根据 route 类型注册不同 handler
4. 配置变更 → `config_watcher` 监听文件变化，优雅重启服务器

## 负载均衡算法

| 算法     | 配置值                 | 说明                       |
| -------- | ---------------------- | -------------------------- |
| 加权轮询 | `weighted_round_robin` | 默认，按权重分配           |
| 轮询     | `round_robin`          | 简单轮询                   |
| IP 哈希  | `iphash`               | 会话保持                   |
| 最少连接 | `least_conn`           | 动态分配到连接最少的服务器 |

## 可选特性

```bash
# 编译时启用/禁用 Lua 支持
cargo build --features lua      # 启用 Lua（默认）
cargo build --no-default-features  # 禁用 Lua
```

## 配置文件示例

```toml
log_level = "info"
log_folder = "./logs"

# 负载均衡上游
[[upstream]]
name = "backend"
method = "weighted_round_robin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },
    { server = "192.168.1.101:8080", weight = 1 }
]

# 虚拟主机
[[host]]
ip = "0.0.0.0"
port = 8080
timeout = 30

[[host.route]]
location = "/"
root = "./html"
auto_index = true

[[host.route]]
location = "/api"
upstream = "backend"
proxy_timeout = 10
```

## 开发注意事项

- **测试隔离**: 测试使用全局状态（HOSTS/UPSTREAMS），必须单线程运行
- **配置验证**: SSL 需要证书文件存在，upstream 引用必须存在
- **热重载**: 修改配置文件后服务器自动重启，无需手动干预
- **调试日志**: 设置 `log_level = "trace"` 或环境变量 `RUST_BACKTRACE=full`
