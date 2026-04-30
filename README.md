# Candy

高性能静态文件 Web 服务器，使用 Rust 编写。

## 特性

- **高性能**: 纯 Rust 实现，零拷贝文件传输
- **HTTP/1.1**: 完整的 HTTP/1.1 支持，包括持久连接和管道化
- **HTTP/2**: 支持 HTTP/2 协议，多路复用
- **TLS/HTTPS**: 基于 rustls 的 TLS 支持
- **压缩**: gzip 压缩协商
- **Range 请求**: 支持断点续传
- **多进程架构**: 基于 SO_REUSEPORT 的多 Worker 进程

## 快速开始

### 构建

```bash
cargo build --release
```

### 运行

```bash
./target/release/candy
```

默认监听 `127.0.0.1:8080`，根目录为当前目录。

### 配置

创建 `candy.toml` 文件：

```toml
[server]
listen = "0.0.0.0:8080"
root = "/var/www"
workers = 4

[http]
keep_alive_timeout = 60

[tls]
enabled = true
cert = "/path/to/cert.pem"
key = "/path/to/key.pem"

[log]
access = true
format = "combined"  # 或 "json"
```

## 配置说明

### `[server]` - 服务器配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `listen` | string | `127.0.0.1:8080` | 监听地址 |
| `root` | string | `.` | 静态文件根目录 |
| `workers` | int | CPU 核心数 | Worker 进程数量 |

### `[http]` - HTTP 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `keep_alive_timeout` | int | `60` | Keep-Alive 超时时间（秒） |

### `[tls]` - TLS 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | bool | `false` | 是否启用 TLS |
| `cert` | string | - | 证书文件路径 |
| `key` | string | - | 私钥文件路径 |

### `[log]` - 日志配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `access` | bool | `true` | 是否记录访问日志 |
| `format` | string | `combined` | 日志格式 (`combined` 或 `json`) |

## 性能

基于 criterion 基准测试：

| 操作 | 延迟 |
|------|------|
| HTTP 请求解析 | ~100-330ns |
| HTTP 响应构建 | ~280-560ns |
| gzip 压缩 (1KB) | ~8.5µs |
| 路径解析 | ~3-7µs |
| Range 解析 | ~33-37ns |

## 架构

```
┌─────────────────────────────────────────┐
│              Master Process              │
│  - 信号处理                              │
│  - Worker 管理                           │
│  - 优雅关闭                              │
└────────────────┬────────────────────────┘
                 │ fork()
    ┌────────────┼────────────┐
    │            │            │
┌───▼───┐    ┌───▼───┐    ┌───▼───┐
│Worker │    │Worker │    │Worker │
│  #1   │    │  #2   │    │  #3   │
└───────┘    └───────┘    └───────┘
     │            │            │
     └────────────┴────────────┘
                  │
         SO_REUSEPORT Socket
```

### 关键组件

- **Master**: 进程管理器，负责 Worker 进程的创建、监控和关闭
- **Worker**: 实际处理 HTTP 请求的工作进程
- **Socket**: SO_REUSEPORT 允许多进程同时监听同一端口
- **HTTP Parser**: 手写的零拷贝 HTTP 解析器
- **sendfile**: Linux sendfile 系统调用实现零拷贝文件传输

## 开发

### 运行测试

```bash
cargo test
```

### 运行基准测试

```bash
cargo bench
```

### 代码结构

```
src/
├── main.rs        # 入口点
├── lib.rs         # 库导出
├── config.rs      # 配置解析
├── master.rs      # Master 进程管理
├── worker.rs      # Worker 请求处理
├── socket.rs      # SO_REUSEPORT Socket
├── http/          # HTTP 解析和响应
│   ├── parser.rs  # HTTP 请求解析器
│   └── response.rs# HTTP 响应构建
├── http2.rs       # HTTP/2 支持
├── tls.rs         # TLS 配置
├── compress.rs    # gzip 压缩
├── sendfile.rs    # 零拷贝文件传输
├── router.rs      # 路径解析和路由
└── logging.rs     # 访问日志
```

## 依赖

- [tokio](https://tokio.rs/) - 异步运行时
- [rustls](https://github.com/rustls/rustls) - TLS 实现
- [nix](https://github.com/nix-rust/nix) - Unix 系统调用
- [flate2](https://github.com/rust-lang/flate2) - gzip 压缩

## 许可证

MIT License
