# Candy

<img src="./assets/candy-transparent.png" width="200px">

一个用 Rust 编写的现代、轻量级 Web 服务器。

[![dependency status](https://deps.rs/repo/github/DefectingCat/candy/status.svg)](https://deps.rs/repo/github/DefectingCat/candy)
![](https://git.rua.plus/xfy/candy/badges/main/pipeline.svg)
![](https://git.rua.plus/xfy/candy/-/badges/release.svg)

## 功能特性

- **静态文件服务** - 提供静态文件服务，支持目录列表
- **反向代理** - 将请求代理到后端服务器，支持轮询负载均衡
- **负载均衡** - 支持 upstream 服务器组配置，提供轮询（Round-Robin）负载均衡
- **Lua 脚本** - 使用 Lua 脚本扩展功能（可选特性）
- **SSL/TLS 加密** - 支持 HTTPS 安全连接
- **HTTP/2 支持** - 现代协议支持，提升性能
- **配置自动重载** - 配置文件变更时自动重载
- **多虚拟主机** - 在单一服务器上托管多个网站
- **单二进制文件** - 无依赖，易于部署

## 快速开始

### 安装

```bash
# 从源代码构建（需要 Rust 环境）
git clone https://github.com/DefectingCat/candy.git
cd candy
cargo build --release
```

### 配置

复制并自定义示例配置：
```bash
cp config.example.toml config.toml
# 编辑 config.toml 以满足您的需求
```

### 运行

```bash
# 使用默认配置运行（config.toml）
cargo run --release

# 或直接运行
./target/release/candy --config path/to/config.toml
```

## 构建命令

```bash
# 构建（调试版）
cargo build

# 构建（发布版）
cargo build --release

# 运行
cargo run -- --config config.toml

# 运行测试
cargo test
```

## 文档

- [配置指南](docs/) - 详细的配置选项
- [示例](examples/) - 使用示例
- [变更日志](CHANGELOG.md) - 版本历史
- [待办列表](TODO.md) - 计划功能

## 许可证

[MIT](LICENSE)
