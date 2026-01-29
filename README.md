# Candy

<img src="./assets/candy-transparent.png" width="200px">

一个用 Rust 编写的现代、轻量级 Web 服务器。

[![dependency status](https://deps.rs/repo/github/DefectingCat/candy/status.svg)](https://deps.rs/repo/github/DefectingCat/candy)
![](https://git.rua.plus/xfy/candy/badges/main/pipeline.svg)
![](https://git.rua.plus/xfy/candy/-/badges/release.svg)

## 功能特性

- **静态文件服务** - 提供静态文件服务，支持目录列表
- **反向代理** - 将请求代理到后端服务器，支持轮询负载均衡
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

## 使用 Makefile

项目提供 Makefile 简化常用操作：

```bash
# 构建（调试版）
make build

# 构建（发布版）
make release

# 运行（调试模式）
make run

# 运行（带参数）
make run ARGS="--config path/to/config.toml"

# 开发模式（自动重载）
make dev

# 运行所有测试
make test

# 代码格式化
make format

# 代码检查
make lint

# 修复常见 lint 问题
make fix

# 检查代码编译
make check
```

## 配置示例

一个简单的配置示例：

```toml
[server]
listen = "0.0.0.0:8080"
workers = 4
log_level = "info"

[virtual_hosts.default]
root = "./html"
index_files = ["index.html", "index.htm"]
directory_listing = true

[virtual_hosts.example]
server_name = "example.com"
root = "./examples/example.com"
index_files = ["index.html"]
```

## 文档

- [配置指南](docs/) - 详细的配置选项说明
- [示例](examples/) - 各种使用场景的配置示例
- [变更日志](CHANGELOG.md) - 版本历史和变更记录
- [待办列表](TODO.md) - 计划开发的功能

## 许可证

[MIT](LICENSE)
