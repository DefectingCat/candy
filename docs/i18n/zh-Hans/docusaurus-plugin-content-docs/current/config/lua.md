---
sidebar_label: Lua 脚本
sidebar_position: 2
title: Lua 脚本
---

## 概述

Candy 支持使用 Lua 脚本作为路由处理方式，允许您编写自定义的 HTTP 请求处理逻辑。Lua 脚本提供了灵活的编程能力，可以与静态文件服务、反向代理等功能配合使用，无需重新编译应用程序。

## 配置方法

在 `config.toml` 中添加路由配置：

```toml
[[host.route]]
location = "/api"
lua_script = "scripts/api_handler.lua"
```

## API 参考

### 全局变量

#### `ctx` - 请求/响应上下文

**请求方法：**
- `ctx:get_path()` - 获取当前请求路径
- `ctx:get_method()` - 获取 HTTP 请求方法

**响应方法：**
- `ctx:set_status(status)` - 设置响应状态码（默认为 200）
- `ctx:set_body(body)` - 设置响应内容
- `ctx:set_header(key, value)` - 设置响应头

#### `candy` - 核心 API

**共享数据：**
- `candy.shared.set(key, value)` - 设置共享数据
- `candy.shared.get(key)` - 获取共享数据

**日志功能：**
- `candy.log(message)` - 输出日志信息（使用 info 级别）

**系统信息：**
- `candy.version` - 获取版本号
- `candy.name` - 获取应用名称
- `candy.os` - 获取操作系统信息
- `candy.arch` - 获取架构信息
- `candy.compiler` - 获取编译器信息
- `candy.commit` - 获取提交哈希

## 示例脚本

### 简单的 Hello World

```lua
-- scripts/hello.lua
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Hello from Lua!")
```

### 访问共享数据

```lua
-- scripts/counter.lua
local count = candy.shared.get("page_count")
count = tonumber(count) or 0
count = count + 1
candy.shared.set("page_count", tostring(count))

ctx:set_body(string.format("Page count: %d", count))
```

### 动态内容生成

```lua
-- scripts/time.lua
local time = os.date("%Y-%m-%d %H:%M:%S")
ctx:set_header("Content-Type", "text/html")
ctx:set_body(string.format([[
    <html>
    <head><title>Current Time</title></head>
    <body>
        <h1>Current Time</h1>
        <p>%s</p>
    </body>
    </html>
]], time))
```

## 最佳实践

1. **保持脚本简洁** - 复杂逻辑应该在 Rust 中实现，Lua 适合处理简单的请求/响应逻辑
2. **注意性能** - Lua 脚本执行会有一定的开销，避免在高并发场景下使用复杂脚本
3. **错误处理** - 脚本中应包含适当的错误处理逻辑
4. **共享数据** - 合理使用共享字典，避免资源竞争
5. **脚本位置** - 建议将 Lua 脚本放在单独的目录中（如 `scripts/` 或 `lua/`）

## 限制

- 不支持异步操作
- 脚本执行有时间限制
- 内存使用有限制
- 不能直接访问底层系统资源
- 不支持 Lua C 扩展
