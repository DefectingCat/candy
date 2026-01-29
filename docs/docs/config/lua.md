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

- `ctx:get_path()` - 获取当前请求路径（字符串）
- `ctx:get_method()` - 获取 HTTP 请求方法（字符串，如 GET、POST）

**响应方法：**

- `ctx:set_status(status)` - 设置响应状态码（数值，默认 200）
- `ctx:set_body(body)` - 设置响应内容（字符串，追加到现有内容）
- `ctx:set_header(key, value)` - 设置响应头（键值对字符串）

#### `candy` - 核心 API

**共享数据：**

- `candy.shared.set(key, value)` - 设置共享数据（键值对字符串）
- `candy.shared.get(key)` - 获取共享数据（返回字符串，不存在返回空字符串）

**日志功能：**

- `candy.log(message)` - 输出日志信息（使用 info 级别）

**系统信息：**

- `candy.version` - 获取版本号（字符串）
- `candy.name` - 获取应用名称（字符串，固定为 "Candy"）
- `candy.os` - 获取操作系统信息（字符串，如 "linux"、"macos"、"windows"）
- `candy.arch` - 获取架构信息（字符串，如 "x86_64"、"aarch64"）
- `candy.compiler` - 获取编译器信息（字符串，如 "rustc 1.70.0"）
- `candy.commit` - 获取提交哈希（字符串，构建时设置）

## API 详细说明

### `ctx` 上下文对象

`ctx` 对象是 Lua 脚本与 HTTP 请求/响应交互的主要接口。

#### 请求方法示例

```lua
-- 获取请求信息
local path = ctx:get_path()
local method = ctx:get_method()

-- 输出到日志
candy.log("Request: " .. method .. " " .. path)
```

#### 响应方法示例

```lua
-- 设置响应状态码和内容
ctx:set_status(200)
ctx:set_header("Content-Type", "application/json")
ctx:set_body('{"message": "Hello from Lua!"}')
```

### `candy.shared` 共享数据存储

共享数据存储是一个线程安全的键值对存储，用于在请求之间共享数据。

```lua
-- 设置共享数据
candy.shared.set("counter", "100")

-- 获取共享数据
local counter = candy.shared.get("counter")
candy.log("Counter: " .. counter)  -- 输出: Counter: 100
```

**共享数据特性：**
- 线程安全
- 全局作用域（整个应用程序共享）
- 只支持字符串类型
- 数据在服务器重启后会丢失

### `candy.log` 日志功能

日志功能用于记录运行时信息。

```lua
-- 记录不同级别的日志（内部使用 info 级别）
candy.log("This is an information message")
candy.log("User " .. username .. " accessed the system")
```

**注意：** 所有日志消息都会被记录为 info 级别。

### 系统信息属性

```lua
-- 获取系统信息
local version = candy.version
local os_info = candy.os
local arch = candy.arch
local compiler = candy.compiler
local commit = candy.commit

-- 构建系统信息字符串
local sys_info = string.format(
    "Version: %s\nOS: %s\nArch: %s\nCompiler: %s\nCommit: %s",
    version, os_info, arch, compiler, commit
)

ctx:set_body(sys_info)
```

## 示例脚本

### 1. 简单的 Hello World

```lua
-- scripts/hello.lua
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Hello from Lua!")
```

### 2. 访问共享数据

```lua
-- scripts/counter.lua
local count = candy.shared.get("page_count")
count = tonumber(count) or 0
count = count + 1
candy.shared.set("page_count", tostring(count))

ctx:set_body(string.format("Page count: %d", count))
```

### 3. 动态内容生成

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

### 4. 请求信息处理

```lua
-- scripts/request_info.lua
local path = ctx:get_path()
local method = ctx:get_method()

local info = string.format([[
    <h1>Request Information</h1>
    <p><strong>Path:</strong> %s</p>
    <p><strong>Method:</strong> %s</p>
    <p><strong>Time:</strong> %s</p>
]], path, method, os.date("%Y-%m-%d %H:%M:%S"))

ctx:set_status(200)
ctx:set_header("Content-Type", "text/html")
ctx:set_body(info)
```

### 5. 简单的 API 响应

```lua
-- scripts/api_response.lua
local response = {
    status = "success",
    message = "Data processed successfully",
    timestamp = os.time(),
    data = {
        user_id = 123,
        username = "test_user",
        email = "test@example.com"
    }
}

-- 简单的 JSON 序列化（手动）
local json = string.format([[
{
    "status": "%s",
    "message": "%s",
    "timestamp": %d,
    "data": {
        "user_id": %d,
        "username": "%s",
        "email": "%s"
    }
}
]], response.status, response.message, response.timestamp,
   response.data.user_id, response.data.username, response.data.email)

ctx:set_status(200)
ctx:set_header("Content-Type", "application/json")
ctx:set_body(json)
```

### 6. 访问控制

```lua
-- scripts/auth.lua
local allowed_ips = {
    ["127.0.0.1"] = true,
    ["192.168.1.100"] = true,
    ["::1"] = true
}

-- 获取客户端 IP（这里简化处理，实际项目中需要更复杂的获取方式）
local client_ip = "127.0.0.1"  -- 需要根据实际请求获取

if not allowed_ips[client_ip] then
    ctx:set_status(403)
    ctx:set_header("Content-Type", "text/plain")
    ctx:set_body("Access denied. Your IP address is not authorized.")
    return
end

-- 允许访问，继续处理
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Access granted. Welcome!")
```

### 7. 错误处理

```lua
-- scripts/error_handling.lua
local success, error_message = pcall(function()
    -- 尝试执行可能失败的操作
    local invalid_table = nil
    local value = invalid_table.field
end)

if not success then
    candy.log("Error: " .. error_message)
    ctx:set_status(500)
    ctx:set_header("Content-Type", "text/plain")
    ctx:set_body("An internal server error occurred.")
    return
end

ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Operation completed successfully.")
```

### 8. 简单的路由逻辑

```lua
-- scripts/router.lua
local path = ctx:get_path()

if path == "/" then
    ctx:set_body("Home Page")
elseif path == "/about" then
    ctx:set_body("About Us")
elseif path == "/contact" then
    ctx:set_body("Contact Page")
else
    ctx:set_status(404)
    ctx:set_body("Page not found")
end

ctx:set_header("Content-Type", "text/plain")
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
