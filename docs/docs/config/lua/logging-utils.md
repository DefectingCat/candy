---
sidebar_label: 日志和工具函数
sidebar_position: 4
title: 日志和工具函数
---

# 日志和工具函数

Candy 的 Lua 脚本提供了丰富的日志记录和工具函数，帮助开发者调试和监控脚本执行。

## 日志记录

### `cd.log(log_level, ...)`

记录日志消息到错误日志。

参数：
- `log_level`：日志级别（使用预定义常量）
- `...`：日志消息参数

```lua
-- 使用不同日志级别
cd.log(cd.ERR, "Error occurred: ", error_msg)
cd.log(cd.WARN, "Warning: ", warning_msg)
cd.log(cd.INFO, "Info: ", info_msg)
cd.log(cd.DEBUG, "Debug: ", debug_msg)

-- 记录多个参数
local user_id = 123
local action = "login"
cd.log(cd.INFO, "User ", user_id, " performed action: ", action)
```

### 日志级别常量

- `cd.EMERG` (2) - 紧急
- `cd.ALERT` (4) - 警报
- `cd.CRIT` (8) - 严重
- `cd.ERR` (16) - 错误
- `cd.WARN` (32) - 警告
- `cd.NOTICE` (64) - 通知
- `cd.INFO` (128) - 信息
- `cd.DEBUG` (255) - 调试

## 工具函数

### `cd.sleep(seconds)`

休眠指定的秒数而不阻塞。

参数：
- `seconds`：休眠秒数（支持小数，精度到毫秒）

```lua
-- 休眠 1 秒
cd.sleep(1)

-- 休眠 0.5 秒（500 毫秒）
cd.sleep(0.5)

-- 休眠 100 毫秒
cd.sleep(0.1)
```

### `cd.escape_uri(str)`

将字符串作为 URI 组件进行转义。

```lua
local original = "hello world & special chars!"
local escaped = cd.escape_uri(original)
cd.print("Original: ", original)
cd.print("Escaped: ", escaped)  -- hello%20world%20%26%20special%20chars%21
```

### `cd.unescape_uri(str)`

将字符串作为转义的 URI 组件进行解码。

```lua
local escaped = "hello%20world%21"
local unescaped = cd.unescape_uri(escaped)
cd.print("Escaped: ", escaped)
cd.print("Unescaped: ", unescaped)  -- hello world!
```

### `cd.encode_args(table)`

将 Lua 表编码为查询参数字符串。

```lua
local args = {
    name = "John Doe",
    age = 30,
    active = true,
    tags = {"tech", "programming", "rust"}
}

local query_string = cd.encode_args(args)
cd.print("Encoded: ", query_string)
-- name=John%20Doe&age=30&active=true&tags=tech&tags=programming&tags=rust
```

### `cd.decode_args(str, max_args?)`

将 URI 编码的查询字符串解码为 Lua 表。

参数：
- `str`：查询字符串
- `max_args`：最大参数数量（默认 100，0 表示无限制）

```lua
local query = "name=Alice&age=25&hobby=coding&hobby=reading"
local args = cd.decode_args(query)

cd.print("Name: ", args["name"])  -- Alice
cd.print("Age: ", args["age"])    -- 25

-- 多值参数
for i, hobby in ipairs(args["hobby"] or {}) do
    cd.print("Hobby ", i, ": ", hobby)
end
```

## 系统信息

### `candy` 模块

Candy 提供了一个全局的 `candy` 模块，包含系统信息：

```lua
-- 获取系统信息
cd.print("Candy Version: ", candy.version)
cd.print("App Name: ", candy.name)
cd.print("OS: ", candy.os)
cd.print("Architecture: ", candy.arch)
cd.print("Compiler: ", candy.compiler)
cd.print("Commit: ", candy.commit)

-- 使用 candy.log 记录日志
candy.log("Application started")
candy.log("Running on ", candy.os, "/", candy.arch)
```

### `candy.log(message)`

记录日志信息（使用 info 级别）。

```lua
-- 记录简单信息
candy.log("Request processed")

-- 记录带变量的信息
local user_id = 123
candy.log("User ", user_id, " accessed the system")
```

### `candy.shared`

共享数据存储，用于在请求之间共享数据。

```lua
-- 设置共享数据
candy.shared.set("counter", "100")

-- 获取共享数据
local counter = candy.shared.get("counter")
cd.print("Counter: ", counter)

-- 递增计数器示例
local current_count = tonumber(candy.shared.get("request_counter")) or 0
current_count = current_count + 1
candy.shared.set("request_counter", tostring(current_count))

cd.print("Request number: ", current_count)
```

## 时间函数

### `cd.now()`

获取当前时间戳（秒，包含毫秒小数部分）。

```lua
local start_time = cd.now()

-- 执行一些操作
-- ...

local end_time = cd.now()
local elapsed = end_time - start_time
cd.log(cd.INFO, "Operation took ", elapsed, " seconds")
```

### `cd.time()`

获取当前时间戳（整数秒）。

```lua
local current_timestamp = cd.time()
cd.print("Current timestamp: ", current_timestamp)
```

### `cd.today()`

获取当前日期（格式：yyyy-mm-dd）。

```lua
local today = cd.today()
cd.print("Today is: ", today)  -- 例如: 2023-12-25
```

## 实用工具示例

### 请求计数器

```lua
-- 实现一个简单的请求计数器
local counter = tonumber(candy.shared.get("total_requests")) or 0
counter = counter + 1
candy.shared.set("total_requests", tostring(counter))

cd.print("Total requests served: ", counter)
```

### 响应时间测量

```lua
local start_time = cd.now()

-- 执行主要逻辑
local result = process_request()

local end_time = cd.now()
local response_time = end_time - start_time

-- 记录性能指标
candy.log("Request processed in ", response_time, " seconds")

-- 在响应中包含性能信息（可选）
cd.header["X-Response-Time"] = tostring(response_time)
cd.print(result)
```

### 用户活动跟踪

```lua
-- 跟踪用户活动
local user_id = get_user_id()  -- 假设这是一个获取用户ID的函数
local last_activity = candy.shared.get("user_" .. user_id .. "_last_active")
candy.shared.set("user_" .. user_id .. "_last_active", tostring(cd.time()))

cd.print("User ", user_id, " activity recorded")
```

### 调试信息收集

```lua
-- 收集调试信息
cd.log(cd.DEBUG, "Request method: ", cd.req.get_method())
cd.log(cd.DEBUG, "Request URI: ", cd.req.get_uri())

local headers = cd.req.get_headers()
for name, value in pairs(headers) do
    cd.log(cd.DEBUG, "Header: ", name, " = ", value)
end

-- 条件性调试输出
if debug_mode then
    local body_data = cd.req.get_body_data()
    if body_data then
        cd.log(cd.DEBUG, "Request body length: ", string.len(body_data))
    end
end
```

## 错误处理和日志

```lua
-- 完整的错误处理示例
local success, result = pcall(function()
    -- 尝试执行可能失败的操作
    local data = risky_operation()
    return data
end)

if not success then
    -- 记录错误
    cd.log(cd.ERR, "Operation failed: ", result)
    
    -- 设置错误响应
    cd.status = 500
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "Internal server error", "message": "]] .. result .. [["}]])
else
    -- 操作成功，记录信息
    cd.log(cd.INFO, "Operation completed successfully")
    cd.print(result)
end
```

这些工具函数为您的 Lua 脚本提供了强大的日志记录、调试和实用功能，帮助您构建健壮且可维护的应用程序。