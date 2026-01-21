# 贡献指南

感谢您有兴趣为 Candy 项目做出贡献！Candy 是一个用 Rust 编写的现代、轻量级 Web 服务器。

## 项目简介

Candy 是一个用 Rust 编写的现代轻量级 Web 服务器，具备以下核心特性：

- 📁 **静态文件服务**：高效托管静态资源
- 🔄 **反向代理**：支持 HTTP/HTTPS 代理转发
- 📜 **Lua 脚本**：可选功能，支持动态请求处理
- 🔒 **SSL/TLS 加密**：内置 HTTPS 支持
- 🔄 **自动配置重载**：配置文件变更时无需重启服务
- 🏠 **多虚拟主机**：支持多个域名部署

## 开发环境设置

### 前置要求

- Rust 1.70+（推荐使用 rustup 安装）
- Cargo（Rust 包管理器）
- make（用于构建脚本）
- Git

### 克隆仓库

```bash
git clone https://github.com/your-username/candy.git
cd candy
```

## 构建和测试

### 构建命令

```bash
# 构建项目（debug 版本）
make build

# 构建 release 版本
make release

# 运行应用
make run

# 清理构建产物
make clean
```

### 测试命令

```bash
# 运行所有测试
make test

# 专门运行 config 模块测试
cargo test --package candy config

# 运行 config watcher 测试
cargo test --package candy config_watcher

# 运行特定测试函数
cargo test test_settings_new --package candy

# 详细输出测试结果
cargo test -v
```

### Linting 和格式化

```bash
# 运行 linter (Clippy)
make lint

# 自动格式化代码 (Rustfmt)
make format

# 自动修复 lint 问题
make fix

# 检查格式化问题（不修改文件）
cargo fmt --check
```

### 交叉编译

项目支持使用 `cross` 进行交叉编译：

```bash
make linux-musl         # x86_64 Linux (musl)
make aarch64-linux-musl # ARM64 Linux (musl)
make aarch64-android    # ARM64 Android
make linux-gnu          # x86_64 Linux (GNU)
make windows-gnu        # x86_64 Windows
make freebsd            # x86_64 FreeBSD
make loongarch          # LoongArch Linux
```

## 代码风格指南

### 通用规则

- **文件编码**: UTF-8
- **行尾**: LF（Unix 风格）
- **尾随空格**: 必须删除
- **文件结尾**: 必须有换行符
- **行长度**: 目标 80-100 字符（软限制）

### Rust 特定规则

- **缩进**: 4 个空格（不使用制表符）- 由 .editorconfig 强制执行
- **导入风格**: 按以下顺序分组和排序：
  1. 标准库 (std::\*)
  2. 外部依赖（按字母顺序）
  3. 内部模块 (crate::_, super::_, self::\*)
  - 使用 `use` 语句导入特定项，避免全局导入
- **命名约定**:
  - 变量/函数: `snake_case`
  - 类型/ trait/枚举: `PascalCase`
  - 常量/静态变量: `SCREAMING_SNAKE_CASE`
  - 模块: `snake_case`
  - 生命周期: `'a`, `'b`（单个小写字母）
- **错误处理**:
  - 使用 `anyhow::Result` 处理应用级错误
  - 使用 `thiserror::Error` 定义结构化错误类型
  - 优先使用 `?` 运算符而不是 `unwrap()`/`expect()`
  - 在生产代码中避免使用 `panic!`（仅用于不可恢复的错误）
  - 使用 `with_context()` 提供上下文以获得更好的错误消息
- **类型注释**:
  - 尽可能使用 Rust 的类型推断
  - 显式注释公共 API 签名
  - 对通用 trait 使用 `derive` 宏 (Debug, Clone, PartialEq, Eq)
- **内存安全**:
  - 优先使用安全 Rust 而不是不安全 Rust
  - 为不安全块添加 `// SAFETY:` 注释

### 文档

- **公共 API**: 必须有文档注释 `///`
- **模块级**: 在模块文件顶部使用 `//!`
- **示例**: 在文档注释中包含可运行的示例
- **错误消息**: 具有描述性和可操作性
- **内部文档**: 对模块级文档使用 `//!`，对内部项使用 `///`

## Git 工作流

### 分支命名

使用 `<类型>/<简短描述>` 格式：

- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档更新
- `refactor`: 代码重构
- `test`: 测试相关
- `chore`: 构建/工具链维护

### 提交规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/v1.0.0/) 规范：

- ✅ 使用祈使语气（"添加功能" 而不是 "添加了功能"）
- ✅ 第一行 <= 50 个字符
- ✅ 正文用空行分隔，解释 "为什么" 而不仅仅是 "做了什么"
- ✅ 引用相关 issue/PR（如 `Fixes #123`）
- ❌ 不要提交密钥或敏感信息

示例：

```
feat: 添加反向代理的负载均衡支持

- 实现轮询和 IP 哈希两种负载均衡算法
- 支持配置权重参数
- Fixes #45
```

### 拉取请求流程

1. 🍴 Fork 仓库并创建自己的分支
2. 💻 提交更改（遵循提交规范）
3. ✅ 运行测试和 linter 确保代码质量
4. 🚀 推送分支到你的 fork
5. 📝 创建拉取请求到主仓库
   - 清晰描述变更内容和解决的问题
   - 关联相关 issue
6. 👀 等待代码审查
7. 🔄 根据审查意见进行修改
8. 🎉 合并到主分支

## 配置

- 项目配置使用 TOML 格式 (`config.example.toml`)
- 复制 `config.example.toml` 到 `config.toml` 并自定义
- 不要提交 `config.toml` 到版本控制
- 配置文件更改时会自动重新加载

## 性能优化

Release 版本包含以下优化：

```toml
[profile.release]
opt-level = 3
strip = true
lto = true
panic = "abort"
codegen-units = 1
```

## 开发工作流

```bash
# 带有实时重载的监视模式
make dev

# 检查编译错误
make check

# 更新依赖
cargo update

# 添加新依赖
cargo add <dependency_name>

# 删除依赖
cargo remove <dependency_name>

# 使用自定义配置文件运行
cargo run -- --config path/to/config.toml
```

## 核心模块

### 主入口

- **src/main.rs**: 初始化日志、加载配置、启动服务器并监视配置更改

### 配置

- **src/config.rs**: 定义配置结构、验证和从 TOML 文件加载

### HTTP 服务器

- **src/http/mod.rs**: 使用 Axum 的核心服务器实现
- **src/http/handler.rs**: 静态文件、代理和 Lua 脚本的请求处理程序
- **src/http/router.rs**: 路由匹配和调度逻辑

### 工具类

- **src/utils/config_watcher.rs**: 监视配置文件更改并重新加载配置
- **src/utils/init_logger.rs**: 初始化跟踪日志
- **src/utils/mime_types.rs**: 静态文件的 MIME 类型检测

### Lua 引擎（可选）

- **src/lua_engine.rs**: Lua 脚本执行上下文（在 "lua" 功能启用时可用）

### 错误处理

- **src/error.rs**: 自定义错误类型和转换函数

## 报告问题

如果您发现错误或有功能请求，请在 GitHub Issues 上创建一个新问题。请遵循以下规范：

### Bug 报告

- 🐛 **标题**：简洁描述问题（如 "静态文件服务返回 404 错误"）
- 📝 **描述**：清晰复现步骤
- 📋 **环境**：Rust 版本、操作系统、Candy 版本
- 📸 **截图**：如果适用，请提供相关截图
- 📋 **日志**：如有错误日志，请一并提供

### 功能请求

- 💡 **标题**：以 "Feature: " 开头（如 "Feature: 添加 WebSocket 支持"）
- 📝 **描述**：详细说明功能需求和使用场景
- 🌟 **价值**：解释该功能对用户的价值

## 获取帮助

如果您有问题或需要帮助，请通过以下方式联系我们：

- 💬 在 GitHub Discussions 上提问

## 行为准则

我们致力于营造一个友好、开放的社区环境。请遵守以下准则：

- 🤝 尊重他人，保持专业
- 📝 清晰表达，耐心倾听
- 🐛 负责任地报告问题
- 🌟 积极贡献，帮助他人

## 许可证

By contributing to Candy, you agree that your contributions will be licensed under the [MIT License](LICENSE).

再次感谢您的贡献！🎉
