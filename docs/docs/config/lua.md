---
sidebar_label: Lua 脚本
sidebar_position: 4
title: Lua 脚本
---

# Lua 脚本

Candy 支持使用 Lua 脚本作为路由处理方式，允许您编写自定义的 HTTP 请求处理逻辑。Candy 的 Lua 实现完全兼容 OpenResty 的 API，使您可以轻松地从 Nginx + Lua 环境迁移现有脚本。

## 概述

Candy 的 Lua 脚本功能提供：

- **OpenResty API 兼容**：支持大部分 OpenResty 的 API
- **高性能**：使用 mlua 库实现，提供高效的 Lua 执行环境
- **安全性**：沙箱执行环境，防止恶意脚本影响服务器
- **可扩展性**：丰富的 API 接口，满足各种复杂需求

## 配置方法

在 `config.toml` 中添加路由配置：

```toml
[[host.route]]
location = "/api"
lua_script = "scripts/api_handler.lua"
lua_code_cache = true  # 启用代码缓存以提高性能
```

## 文档结构

本 Lua 脚本文档分为以下几个部分：

1. [Lua 脚本入门](./config/lua/intro.md) - Lua 脚本的基本概念和快速开始
2. [请求 API](./config/lua/request-api.md) - 详细的请求处理 API 文档
3. [响应 API](./config/lua/response-api.md) - 详细的响应处理 API 文档
4. [日志和工具函数](./config/lua/logging-utils.md) - 日志记录和实用工具函数
5. [实际应用示例](./config/lua/examples.md) - 各种实际应用场景的示例代码
6. [性能优化与最佳实践](./config/lua/performance-best-practices.md) - 性能优化和最佳实践指南

## 主要特性

- **OpenResty API 兼容**：支持 `cd.req`、`cd.resp`、`cd.header` 等对象
- **代码缓存**：支持 Lua 代码缓存以提高性能
- **共享数据**：通过 `candy.shared` 实现跨请求数据共享
- **日志系统**：集成的多级别日志记录功能
- **请求/响应操作**：完整的请求和响应处理能力

## 限制

- 不支持异步操作
- 脚本执行有时间限制
- 内存使用有限制
- 不能直接访问底层系统资源
- 不支持 Lua C 扩展
