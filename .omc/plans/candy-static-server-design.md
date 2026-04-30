# Candy 高性能静态文件服务器 - 实现方案

## 项目概述

**目标**: 构建一个生产级、超高性能的静态文件服务器，支持 HTTP/1.1 和 HTTP/2。

**核心原则**:
- 最小依赖（仅 tokio + http + rustls + toml）
- 能手写的全部手写
- 性能优先，追求极致吞吐

---

## 技术架构

### 整体架构

```
                    ┌─────────────────────────────────────┐
                    │           Master Process            │
                    │  - 解析配置                          │
                    │  - 创建 worker 进程                  │
                    │  - 监控/重启 worker                  │
                    └──────────────┬──────────────────────┘
                                   │ spawn
          ┌────────────────────────┼────────────────────────┐
          │                        │                        │
          ▼                        ▼                        ▼
    ┌───────────┐           ┌───────────┐           ┌───────────┐
    │  Worker 0 │           │  Worker 1 │           │  Worker N │
    │           │           │           │           │           │
    │ ┌───────┐ │           │ ┌───────┐ │           │ ┌───────┐ │
    │ │ HTTP  │ │           │ │ HTTP  │ │           │ │ HTTP  │ │
    │ │ Parser│ │           │ │ Parser│ │           │ │ Parser│ │
    │ └───┬───┘ │           │ └───┬───┘ │           │ └───┬───┘ │
    │     │     │           │     │     │           │     │     │
    │ ┌───┴───┐ │           │ ┌───┴───┐ │           │ ┌───┴───┐ │
    │ │Router │ │           │ │Router │ │           │ │Router │ │
    │ └───┬───┘ │           │ └───┬───┘ │           │ └───┬───┘ │
    │     │     │           │     │     │           │     │     │
    │ ┌───┴───┐ │           │ ┌───┴───┐ │           │ ┌───┴───┐ │
    │ │sendfile│ │           │ │sendfile│ │           │ │sendfile│ │
    │ └───────┘ │           │ └───────┘ │           │ └───────┘ │
    └─────┬─────┘           └─────┬─────┘           └─────┬─────┘
          │                       │                       │
          └───────────────────────┴───────────────────────┘
                                  │
                          SO_REUSEPORT
                          (内核负载均衡)
```

### 核心组件

| 组件 | 职责 | 实现方式 |
|------|------|----------|
| **Master** | 配置解析、进程管理、监控 | 手写，基于 `nix` crate |
| **Worker** | 接受连接、处理请求、响应 | 基于 tokio runtime |
| **HTTP/1.1 Parser** | 解析请求、构建响应 | 手写状态机，零拷贝 |
| **HTTP/2 Handler** | 帧解析、多路复用、流管理 | 手写，HPACK 压缩 |
| **TLS** | 握手、加解密 | rustls |
| **Router** | URL 路径映射到文件系统 | 手写，规范化路径 |
| **sendfile** | 零拷贝文件传输 | syscall 封装 |
| **Logger** | 请求日志 | 异步写入，缓冲区 |

---

## 目录结构

```
candy/
├── Cargo.toml
├── candy.toml              # 配置文件
├── src/
│   ├── main.rs             # 入口，master 进程
│   ├── config.rs           # TOML 配置解析
│   ├── master.rs           # Master 进程逻辑
│   ├── worker.rs           # Worker 进程逻辑
│   ├── http/
│   │   ├── mod.rs
│   │   ├── parser.rs       # HTTP/1.1 解析器
│   │   ├── response.rs     # HTTP 响应构建
│   │   ├── h2.rs           # HTTP/2 帧处理
│   │   └── hpack.rs        # HPACK 头部压缩
│   ├── tls.rs              # rustls 集成
│   ├── router.rs           # 路径路由
│   ├── sendfile.rs         # sendfile syscall
│   ├── log.rs              # 请求日志
│   └── util/
│       ├── mod.rs
│       ├── pool.rs         # 对象池
│       └── buffer.rs       # 缓冲区管理
└── tests/
    ├── integration/
    └── benchmark/
```

---

## 依赖清单

```toml
[package]
name = "candy"
version = "0.1.0"
edition = "2021"

[dependencies]
# 异步运行时
tokio = { version = "1", features = ["rt-multi-thread", "net", "io-util", "signal", "sync"] }

# HTTP 类型
http = "1"
http-body = "1"
bytes = "1"

# TLS
rustls = "0.23"
rustls-pemfile = "2"

# 配置
toml = "0.8"
serde = { version = "1", features = ["derive"] }

# 系统调用
nix = { version = "0.29", features = ["process", "signal", "socket"] }
libc = "0.2"

# 日志
chrono = "0.4"

[dev-dependencies]
criterion = "0.5"
reqwest = { version = "0.12", features = ["rustls-tls"] }
```

---

## 核心模块设计

### 1. 配置系统 (`config.rs`)

```toml
# candy.toml 示例
[server]
listen = "0.0.0.0:8080"
workers = 4
root = "/var/www"

[tls]
enabled = true
cert = "/etc/candy/cert.pem"
key = "/etc/candy/key.pem"

[http]
max_header_size = 8192
keep_alive_timeout = 60

[log]
access = true
format = "combined"  # 或 "json"
```

### 2. SO_REUSEPORT Worker 池 (`master.rs` + `worker.rs`)

**Master 职责**:
- 解析配置
- fork N 个 worker 进程
- 监控 worker 存活，异常时重启
- 处理信号（SIGHUP 重载配置，SIGTERM 优雅关闭）

**Worker 职责**:
- 独立创建 socket，设置 SO_REUSEPORT
- 运行 tokio runtime
- 接受连接、处理请求

```rust
// 伪代码
fn worker_main(config: &Config) {
    let listener = create_reuseport_socket(&config.listen);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            loop {
                let (stream, _) = listener.accept().await;
                tokio::spawn(handle_connection(stream));
            }
        });
}
```

### 3. HTTP/1.1 解析器 (`http/parser.rs`)

**设计要点**:
- 状态机解析，零拷贝
- 不使用 `httparse` 等 crate，手写
- 支持：持久连接、管道化、chunked、range requests

```rust
enum ParseState {
    Method,
    Path,
    Version,
    HeaderName,
    HeaderValue,
    Body,
    Complete,
}

struct Request {
    method: Method,
    path: Bytes,
    version: HttpVersion,
    headers: Vec<(Bytes, Bytes)>,
    body: Option<Bytes>,
}
```

### 4. HTTP/2 处理器 (`http/h2.rs`)

**设计要点**:
- ALPN 协商后切换到 HTTP/2
- 帧解析：DATA, HEADERS, PRIORITY, RST_STREAM, SETTINGS, PUSH_PROMISE, PING, GOAWAY, WINDOW_UPDATE, CONTINUATION
- 流状态机：idle, reserved, active, closed
- 流量控制：窗口管理
- HPACK 头部压缩

```rust
struct H2Connection {
    streams: HashMap<u32, Stream>,
    send_window: u32,
    recv_window: u32,
    hpack_encoder: HpackEncoder,
    hpack_decoder: HpackDecoder,
}

struct Stream {
    id: u32,
    state: StreamState,
    send_window: u32,
    recv_window: u32,
}
```

### 5. sendfile 封装 (`sendfile.rs`)

**Linux sendfile syscall**:
```rust
use libc::{sendfile, off_t};

pub fn send_file(fd: i32, socket: i32, offset: u64, count: usize) -> io::Result<usize> {
    let mut off = offset as off_t;
    let sent = unsafe {
        sendfile(socket, fd, &mut off, count)
    };
    if sent < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(sent as usize)
    }
}
```

**异步集成**:
```rust
pub async fn send_file_async(file: &File, socket: &TcpStream) -> io::Result<()> {
    // 获取文件描述符
    let fd = file.as_raw_fd();
    let sock = socket.as_raw_fd();

    // 获取文件大小
    let meta = file.metadata().await?;
    let size = meta.len() as usize;

    // 非阻塞 sendfile
    loop {
        match send_file(fd, sock, 0, size) {
            Ok(n) if n > 0 => return Ok(()),
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                socket.writable().await?;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
```

### 6. 路由器 (`router.rs`)

**职责**:
- URL 路径规范化（防止目录穿越）
- 映射到文件系统路径
- 处理 index 文件、目录列表

```rust
pub fn resolve_path(base: &Path, url: &str) -> Option<PathBuf> {
    // 规范化：移除 . 和 ..
    let normalized = normalize_path(url);

    // 拼接基础路径
    let full = base.join(normalized.trim_start_matches('/'));

    // 安全检查：确保在 base 目录内
    if !full.starts_with(base) {
        return None;
    }

    // 处理目录：尝试 index.html
    if full.is_dir() {
        let index = full.join("index.html");
        if index.exists() {
            return Some(index);
        }
    }

    Some(full)
}
```

### 7. 请求日志 (`log.rs`)

**格式** (Nginx combined 风格):
```
192.168.1.1 - - [30/Apr/2026:10:00:00 +0800] "GET /index.html HTTP/1.1" 200 1234 "-" "Mozilla/5.0"
```

**实现**:
- 异步写入，缓冲区批量 flush
- 每个请求完成后记录

---

## 性能优化策略

### 1. 零拷贝
- HTTP 解析：`Bytes` 引用原始缓冲区
- 文件传输：`sendfile` 内核直接传输
- 避免不必要的 `clone()`

### 2. 内存管理
- 缓冲区池：复用 `BytesMut`
- 对象池：复用 `Request`/`Response` 对象
- 避免频繁分配

### 3. 并发优化
- SO_REUSEPORT：无锁 accept
- 每个 worker 独立状态：无共享竞争
- CPU 亲和性：worker 绑定核心

### 4. I/O 优化
- 批量写入：日志缓冲区
- TCP_NODELAY：禁用 Nagle 算法
- 适当的缓冲区大小（16KB-64KB）

---

## 实现阶段

### Phase 1: 基础框架
1. 配置解析
2. Master/Worker 进程模型
3. SO_REUSEPORT socket
4. 基础 HTTP/1.1 解析（GET/HEAD）
5. 静态文件响应（read/write）

### Phase 2: HTTP/1.1 完善
1. 完整请求解析（所有方法、头部）
2. 持久连接、管道化
3. Range requests
4. Chunked transfer
5. 压缩协商（gzip）

### Phase 3: TLS + HTTP/2
1. rustls 集成
2. ALPN 协商
3. HTTP/2 帧解析
4. 多路复用流
5. HPACK 压缩
6. 流量控制

### Phase 4: 生产就绪
1. 请求日志
2. 信号处理（reload/shutdown）
3. 错误页面
4. 性能测试
5. 文档

---

## 验收标准

| 指标 | 目标 |
|------|------|
| **吞吐量** | > 100K RPS（静态小文件） |
| **延迟 P99** | < 10ms（本地回环） |
| **内存** | < 50MB（空闲） |
| **依赖数** | < 15 个 crate |
| **HTTP/1.1** | 完整支持 |
| **HTTP/2** | 完整支持 |
| **TLS** | rustls 集成 |

---

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| HTTP/2 复杂度高 | 分阶段实现，先保证核心功能 |
| sendfile 异步集成 | 使用 tokio 异步封装，处理 WouldBlock |
| 进程管理复杂 | 参考 Nginx 设计，充分测试信号处理 |
| 性能目标未达标 | 持续 profiling，针对性优化 |

---

## 下一步

方案确认后，将生成详细的实现计划，包括：
- 每个阶段的具体任务
- 文件级别的实现细节
- 测试策略
