# Candy 实现计划

## 概述

基于已确认的设计方案，本计划将实现分解为可执行的任务单元。

---

## RALPLAN-DR 摘要

### 原则
1. **最小依赖** — 仅使用必要的 crate，能手写的全部手写
2. **性能优先** — 零拷贝、无锁架构、内核级优化
3. **渐进实现** — 分阶段交付，每阶段可验证
4. **生产就绪** — 完善的错误处理、日志、信号管理

### 决策驱动因素
1. **极致性能目标** (>100K RPS) — 决定了 SO_REUSEPORT + sendfile 架构
2. **纯 Rust 栈** — 决定了 rustls 而非 OpenSSL
3. **HTTP/2 支持** — 增加了 HPACK、流多路复用等复杂度

### 可选方案

| 方案 | 优点 | 缺点 |
|------|------|------|
| **A. SO_REUSEPORT 多进程** (已选) | 无锁竞争、内核负载均衡、故障隔离 | 进程管理复杂 |
| B. 单进程多线程 | 实现简单 | 有锁竞争、无法利用多核扩展 |
| C. 线程池共享 listener | 中等复杂度 | accept 锁成为瓶颈 |

---

## Phase 1: 基础框架

**目标**: 跑通最小可用版本，验证架构可行性

### Task 1.1: 项目初始化与依赖配置
**文件**: `Cargo.toml`
**内容**:
- 配置依赖项（tokio, http, bytes, rustls, toml, serde, nix, libc, chrono）
- 配置 release 优化选项（LTO, codegen-units=1）

**验收**:
- [ ] `cargo build --release` 成功
- [ ] 依赖数 < 15 个

### Task 1.2: 配置系统
**文件**: `src/config.rs`, `candy.toml`
**内容**:
- 定义 `Config` 结构体
- TOML 解析
- 配置验证（路径存在性、端口有效性）

```rust
pub struct Config {
    pub server: ServerConfig,
    pub tls: Option<TlsConfig>,
    pub http: HttpConfig,
    pub log: LogConfig,
}
```

**验收**:
- [ ] 解析示例配置成功
- [ ] 无效配置返回明确错误

### Task 1.3: SO_REUSEPORT Socket 封装
**文件**: `src/worker.rs`
**内容**:
- 创建 TCP socket
- 设置 SO_REUSEPORT + SO_REUSEADDR
- 绑定并监听

```rust
fn create_reuseport_socket(addr: &SocketAddr) -> io::Result<TcpListener> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    socket.set_reuse_port(true)?;
    socket.set_reuse_address(true)?;
    socket.bind(&SockAddr::from(*addr))?;
    socket.listen(1024)?;
    Ok(TcpListener::from_std(socket.into())?)
}
```

**验收**:
- [ ] 多个进程可同时绑定同一端口
- [ ] 单进程可正常 accept 连接

### Task 1.4: Master 进程管理
**文件**: `src/master.rs`, `src/main.rs`
**内容**:
- fork N 个 worker 进程
- 维护 worker PID 列表
- 信号处理框架（SIGTERM, SIGINT）

**验收**:
- [ ] 启动后 `ps` 可见 N+1 个进程
- [ ] SIGTERM 可终止所有进程

### Task 1.5: 基础 HTTP/1.1 解析器
**文件**: `src/http/mod.rs`, `src/http/parser.rs`
**内容**:
- 状态机解析请求行（GET/HEAD）
- 解析常用头部（Host, Connection, User-Agent）
- 零拷贝设计（Bytes 引用）

```rust
pub struct Request {
    pub method: Method,
    pub path: Bytes,
    pub version: HttpVersion,
    pub headers: Vec<(Bytes, Bytes)>,
}
```

**验收**:
- [ ] 解析有效 GET 请求成功
- [ ] 无效请求返回错误
- [ ] 无内存拷贝（通过 benchmark 验证）

### Task 1.6: 基础响应构建
**文件**: `src/http/response.rs`
**内容**:
- 构建响应头
- Content-Type 推断（MIME）
- Content-Length 设置

**验收**:
- [ ] 响应格式符合 HTTP/1.1 规范
- [ ] curl 可正常接收响应

### Task 1.7: 文件读取与响应（基础版）
**文件**: `src/router.rs`, `src/worker.rs`
**内容**:
- 路径规范化（防目录穿越）
- 使用 tokio::fs 读取文件
- 返回文件内容

**验收**:
- [ ] GET /index.html 返回正确内容
- [ ] GET /../../../etc/passwd 返回 403
- [ ] 不存在的文件返回 404

### Task 1.8: Worker 事件循环
**文件**: `src/worker.rs`
**内容**:
- tokio runtime 初始化
- accept 循环
- 连接处理 spawn

**验收**:
- [ ] 可并发处理多个连接
- [ ] 单连接关闭不影响其他连接

---

## Phase 2: HTTP/1.1 完善

**目标**: 完整的 HTTP/1.1 支持，性能优化

### Task 2.1: 完整请求解析
**文件**: `src/http/parser.rs`
**内容**:
- 支持所有方法（POST, PUT, DELETE, OPTIONS, HEAD）
- 完整头部解析
- Body 读取（Content-Length）

**验收**:
- [ ] 所有标准方法可解析
- [ ] 大头部（>8KB）正确处理

### Task 2.2: 持久连接与管道化
**文件**: `src/http/parser.rs`, `src/worker.rs`
**内容**:
- Connection: keep-alive 处理
- 多请求复用连接
- Keep-Alive 超时

**验收**:
- [ ] 单连接可发送多个请求
- [ ] 超时后连接关闭

### Task 2.3: Range Requests
**文件**: `src/http/range.rs`
**内容**:
- Range 头解析
- 206 Partial Content 响应
- 多段 range 支持

**验收**:
- [ ] `curl -H "Range: bytes=0-99"` 返回前 100 字节
- [ ] 无效 range 返回 416

### Task 2.4: Chunked Transfer Encoding
**文件**: `src/http/response.rs`
**内容**:
- chunked 写入
- 动态内容支持

**验收**:
- [ ] 大文件正确分块传输
- [ ] 客户端正确重组

### Task 2.5: sendfile 零拷贝
**文件**: `src/sendfile.rs`
**内容**:
- Linux sendfile syscall 封装
- 异步集成（处理 WouldBlock）
- 与 tokio TcpStream 集成

**验收**:
- [ ] 文件传输无用户态拷贝
- [ ] 性能对比 read/write 提升 >30%

### Task 2.6: 压缩协商
**文件**: `src/http/compress.rs`
**内容**:
- Accept-Encoding 解析
- gzip 压缩响应
- 预压缩文件支持（.gz）

**验收**:
- [ ] Accept-Encoding: gzip 返回压缩内容
- [ ] 响应 Content-Encoding 正确

### Task 2.7: 目录索引
**文件**: `src/router.rs`
**内容**:
- 目录请求处理
- index.html 自动查找
- 目录列表（可选）

**验收**:
- [ ] / 重定向到 /index.html
- [ ] 目录不存在 index.html 返回 403 或列表

---

## Phase 3: TLS + HTTP/2

**目标**: 安全连接与 HTTP/2 支持

### Task 3.1: rustls 集成
**文件**: `src/tls.rs`
**内容**:
- 证书/私钥加载
- TLS Acceptor 配置
- ALPN 协商

**验收**:
- [ ] HTTPS 连接成功
- [ ] 证书验证通过

### Task 3.2: HTTP/2 帧解析
**文件**: `src/http/h2.rs`
**内容**:
- 帧头解析（9 字节）
- SETTINGS, HEADERS, DATA, RST_STREAM, PING, GOAWAY 帧
- 帧序列化

```rust
pub struct Frame {
    pub length: u32,
    pub frame_type: FrameType,
    pub flags: u8,
    pub stream_id: u32,
    pub payload: Bytes,
}
```

**验收**:
- [ ] 所有帧类型可解析
- [ ] 帧序列化后可被客户端识别

### Task 3.3: HTTP/2 流管理
**文件**: `src/http/h2.rs`
**内容**:
- 流状态机（idle, open, closed）
- 并发流限制
- 流优先级（可选）

**验收**:
- [ ] 多流并发正确
- [ ] 流限制生效

### Task 3.4: HPACK 头部压缩
**文件**: `src/http/hpack.rs`
**内容**:
- 静态表
- 动态表
- Huffman 编码

**验收**:
- [ ] 头部压缩率 > 80%
- [ ] 解码后与原始一致

### Task 3.5: HTTP/2 流量控制
**文件**: `src/http/h2.rs`
**内容**:
- WINDOW_UPDATE 处理
- 连接级/流级窗口

**验收**:
- [ ] 大文件传输不阻塞其他流
- [ ] 窗口耗尽时正确等待

### Task 3.6: HTTP/2 多路复用
**文件**: `src/http/h2.rs`
**内容**:
- 多请求并发处理
- 响应帧交错发送

**验收**:
- [ ] 单连接并发 100 请求成功
- [ ] 队头阻塞不影响

---

## Phase 4: 生产就绪

**目标**: 完善运维功能，性能验证

### Task 4.1: 请求日志
**文件**: `src/log.rs`
**内容**:
- Nginx combined 格式
- 异步写入
- 缓冲区批量 flush

**验收**:
- [ ] 日志格式正确
- [ ] 高并发下不阻塞

### Task 4.2: 信号处理完善
**文件**: `src/master.rs`
**内容**:
- SIGHUP: 重载配置
- SIGTERM/SIGINT: 优雅关闭
- SIGUSR1: 重开日志

**验收**:
- [ ] 信号处理正确
- [ ] 优雅关闭等待连接完成

### Task 4.3: 错误页面
**文件**: `src/http/response.rs`, `error_pages/`
**内容**:
- 自定义错误页面
- 默认错误页面

**验收**:
- [ ] 404/500 页面可自定义
- [ ] 默认页面友好

### Task 4.4: 性能测试
**文件**: `tests/benchmark/`
**内容**:
- wrk/ab 压测脚本
- 性能指标收集

**验收**:
- [ ] > 100K RPS（静态小文件）
- [ ] P99 延迟 < 10ms

### Task 4.5: 集成测试
**文件**: `tests/integration/`
**内容**:
- HTTP/1.1 完整测试
- HTTP/2 完整测试
- TLS 测试
- 边界条件测试

**验收**:
- [ ] 测试覆盖率 > 80%
- [ ] 所有测试通过

### Task 4.6: 文档
**文件**: `README.md`, `docs/`
**内容**:
- 安装说明
- 配置文档
- 性能调优指南

**验收**:
- [ ] 文档完整
- [ ] 示例可运行

---

## 验收标准汇总

| 指标 | 目标 | 验证方式 |
|------|------|----------|
| 吞吐量 | > 100K RPS | wrk 压测 |
| 延迟 P99 | < 10ms | wrk 延迟分布 |
| 内存占用 | < 50MB 空闲 | top/htop |
| 依赖数 | < 15 crate | cargo tree |
| HTTP/1.1 | 完整支持 | 测试套件 |
| HTTP/2 | 完整支持 | 测试套件 |
| TLS | rustls 集成 | curl https 测试 |

---

## 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| HTTP/2 复杂度超预期 | 中 | 高 | 分阶段实现，先核心功能 |
| sendfile 异步集成困难 | 低 | 中 | 参考 tokio-uring 实现 |
| 性能目标未达标 | 中 | 高 | 持续 profiling，针对性优化 |
| 进程管理 bug | 中 | 高 | 充分测试信号处理场景 |

---

## ADR (Architecture Decision Record)

### Decision
采用 SO_REUSEPORT 多进程架构 + sendfile 零拷贝 + 手写 HTTP 解析器

### Drivers
1. 极致性能目标 (>100K RPS)
2. 纯 Rust 技术栈
3. 最小依赖原则

### Alternatives Considered
1. **单进程多线程**: 实现简单，但无法避免锁竞争
2. **共享 listener 线程池**: 中等复杂度，accept 成为瓶颈
3. **使用 hyper/actix-web**: 成熟稳定，但依赖多、控制粒度粗

### Why Chosen
SO_REUSEPORT 是 Nginx 验证过的高性能架构，配合 sendfile 可实现零拷贝文件传输，手写解析器避免依赖开销。三者结合最大化性能潜力。

### Consequences
- 进程管理复杂度增加
- HTTP/2 实现工作量较大
- 需要深入理解 Linux 内核网络栈

### Follow-ups
- Phase 1 完成后进行性能基线测试
- Phase 3 完成后进行 HTTP/2 兼容性测试
- 最终版本进行生产环境验证
