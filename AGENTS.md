# Candy 项目交接文档

## 项目概述

**Candy** 是一个用 Rust 编写的现代、轻量级 Web 服务器（版本 0.2.4）。它提供了静态文件服务、反向代理、负载均衡、Lua 脚本支持等功能，是一个高性能且易于配置的服务器解决方案。

## 核心功能

### 主要特性

- 静态文件服务，支持目录列表
- 反向代理，支持负载均衡
- Lua 脚本支持（可选功能）
- SSL/TLS 加密（HTTPS），支持 HTTP/2
- 配置文件变更自动重载
- 多虚拟主机支持
- 正向代理支持
- HTTP 重定向处理
- 自定义错误页面

### 技术栈

- **Web 框架**: Axum（异步、高性能）
- **服务器**: Axum Server（HTTP/1.1 + HTTP/2）
- **异步运行时**: Tokio
- **日志**: Tracing
- **配置**: Serde + TOML
- **压缩**: Axum 压缩中间件（gzip、deflate、brotli、zstd）
- **Lua 支持**: Mlua（可选）

## 项目结构

```
/Users/xfy/Developer/candy/
├── src/                     # 源代码目录
│   ├── main.rs              # 入口点，服务器生命周期管理
│   ├── config.rs            # 配置加载、验证和结构体定义
│   ├── cli.rs               # 命令行参数解析
│   ├── consts.rs            # 常量定义（版本、构建信息、默认值）
│   ├── error.rs             # 自定义错误类型
│   ├── http/                # HTTP 相关模块
│   │   ├── mod.rs           # 服务器创建和路由注册
│   │   ├── serve.rs         # 静态文件服务
│   │   ├── reverse_proxy.rs # 反向代理实现
│   │   ├── forward_proxy.rs # 正向代理实现
│   │   ├── redirect.rs      # 重定向处理
│   │   ├── lua.rs           # Lua 脚本集成（可选）
│   │   └── error.rs         # HTTP 特定错误类型
│   ├── utils/               # 工具模块
│   │   ├── mod.rs           # 工具模块入口
│   │   ├── config_watcher.rs # 配置文件监听（自动重载）
│   │   ├── logging.rs       # 日志初始化
│   │   └── service.rs       # 服务工具
│   ├── middlewares/         # Axum 中间件实现
│   └── lua_engine.rs        # Lua 引擎初始化（可选特性）
├── examples/                # 示例配置文件
├── docs/                    # 文档
├── assets/                  # 静态资源
├── Cargo.toml               # Rust 项目配置
├── Cargo.lock               # 依赖锁定文件
├── Makefile                 # 构建脚本
├── CLAUDE.md                # 开发规则和架构说明
└── README.md                # 项目说明文档
```

## 快速上手

### 构建项目

```bash
# 调试构建
make build          # 或 cargo build
make run            # 或 cargo run

# 发布构建
make release        # cargo build --release

# 代码格式化和检查
make format         # 格式化代码
make lint           # 运行 Clippy 检查
make fix            # 自动修复 lint 问题并格式化
```

### 运行服务器

```bash
# 使用默认配置文件（config.toml）
candy

# 使用指定配置文件
candy -c /path/to/config.toml

# 查看帮助信息
candy --help
```

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
method = "weighted_round_robin"  # 负载均衡方法：round_robin/weighted_round_robin/ip_hash

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

# HTTPS 虚拟主机示例
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./cert.pem"
certificate_key = "./key.pem"
timeout = 30

[[host.route]]
location = "/"
root = "./html/ssl"
error_page = { status = 404, page = "/404.html" }
```

## 核心模块详解

### 入口点：src/main.rs

- 解析命令行参数
- 加载和验证配置
- 初始化日志系统
- 启动服务器
- 管理服务器生命周期（启动/停止/关闭）
- 监听配置文件变更，自动重载

### 配置管理：src/config.rs

- 使用 Serde 反序列化配置
- 配置验证逻辑
- 上游服务器和虚拟主机配置解析
- 配置字段默认值
- 配置验证测试

### 服务器创建：src/http/mod.rs

- `make_server`：根据配置创建服务器
- 路由注册
- 主机和上游配置存储
- 服务器生命周期管理

### 静态文件服务：src/http/serve.rs

- 处理静态文件请求
- 目录列表支持
- MIME 类型检测
- 错误处理（文件未找到等）

### 反向代理：src/http/reverse_proxy.rs

- 负载均衡实现
- 支持轮询、加权轮询、IP 哈希算法
- 请求/响应头处理
- 超时管理

### 正向代理：src/http/forward_proxy.rs

- HTTP 代理实现
- 客户端请求转发

### 重定向：src/http/redirect.rs

- 路由级别的重定向处理
- 支持 HTTP 状态码 301/302

### 配置监听：src/utils/config_watcher.rs

- 使用 notify 库监听配置文件变化
- 触发服务器重启
- 处理配置重载过程中的错误

### Lua 引擎：src/lua_engine.rs（可选）

- Lua 脚本引擎初始化
- 提供请求/响应处理的 Lua API
- 与 Axum 请求处理程序集成

## 开发流程

### 开发规则

1. **代码检查**：使用 Clippy 进行 lint 检查

   ```bash
   make lint  # 或 cargo clippy
   ```

2. **代码格式化**：使用 rustfmt 格式化代码

   ```bash
   make format  # 或 cargo fmt
   ```

3. **自动修复**：自动修复 lint 问题并格式化

   ```bash
   make fix
   ```

4. **运行测试**：运行所有测试
   ```bash
   make test  # 或 cargo test
   ```

### 函数注释要求

所有 Rust 函数必须遵循以下注释格式：

```rust
/// 函数功能描述
///
/// # 参数
///
/// * `parameter1` - 参数1的详细描述
/// * `parameter2` - 参数2的详细描述
///
/// # 类型参数（对于泛型函数）
///
/// * `T` - 类型参数T的详细描述
/// * `E` - 类型参数E的详细描述
///
/// # 返回值
///
/// 详细描述返回值
fn function_name() { ... }
```

**要求：**

- 所有公共函数和内部重要函数必须有注释
- 注释应清晰描述函数功能
- 每个参数必须有详细说明
- 对于泛型函数，必须解释类型参数
- 返回值必须明确描述
- 使用 Markdown 格式的 `# 参数`、`# 类型参数` 和 `# 返回值` 标题

**示例：**

```rust
/// 处理单个配置文件事件
///
/// # 参数
///
/// * `result` - 通知库返回的事件结果（可能包含错误）
/// * `is_processing` - 是否正在处理事件的原子标志
/// * `last_event_time` - 上一次处理事件的时间戳
/// * `debounce_duration` - 防抖时间间隔
/// * `config_path` - 配置文件路径
/// * `watcher` - 配置文件监听器实例
/// * `callback` - 配置变化时的回调函数
/// * `config` - 监听器配置参数
///
/// # 返回值
///
/// 返回操作结果，成功或包含错误信息
async fn process_event(result: Option<std::result::Result<notify::Event, notify::Error>>, ...) -> Result<(), notify::Error> {
    // 函数实现
}
```

### 调试

- 使用 `RUST_BACKTRACE=full` 环境变量获取完整堆栈跟踪
- 使用 `log_level = "trace"` 在配置文件中启用详细日志
- 使用 `cargo run -- --help` 查看命令行选项

## 常见操作

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

### 部署

```bash
# 构建发布版本
make release

# 运行服务器
./target/release/candy -c /path/to/config.toml
```

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

## 技术债务与改进建议

### 当前限制

- 缺少完整的集成测试
- 文档需要进一步完善
- 部分模块缺乏详细的单元测试
- 配置验证逻辑可以更严格

### 改进方向

1. **测试覆盖**：增加单元测试和集成测试
2. **文档完善**：为所有公共 API 添加文档注释
3. **性能优化**：考虑使用其他异步运行时或优化内存分配
4. **功能增强**：添加更多中间件、缓存支持、健康检查等功能

## 联系方式

项目维护者：rua.plus

## 许可证

MIT 许可证 - 详见 LICENSE 文件
