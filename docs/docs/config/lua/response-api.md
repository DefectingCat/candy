---
sidebar_label: 响应 API
sidebar_position: 3
title: 响应 API
---

# 响应 API

Candy 的 Lua 脚本提供了全面的响应处理 API，通过 `cd` 对象和 `cd.header` 对象访问。这些 API 与 OpenResty 的 `ngx.*` 系列函数兼容。

## 设置响应状态

### `cd.status`

设置响应的 HTTP 状态码。

```lua
-- 设置成功状态
cd.status = 200

-- 设置其他状态码
cd.status = 404  -- Not Found
cd.status = 500  -- Internal Server Error
cd.status = 302  -- Moved Temporarily
```

## 响应内容输出

### `cd.print(...)`

输出数据到响应体，连接所有参数并发送到 HTTP 客户端。

```lua
-- 输出简单文本
cd.print("Hello, World!")

-- 输出多个参数
cd.print("User: ", "Alice", ", Age: ", 25)

-- 输出表格内容
local user = {name = "Bob", age = 30}
cd.print("User: ", user.name, ", Age: ", user.age)
```

### `cd.say(...)`

输出数据到响应体并添加换行符。

```lua
-- 输出带换行的文本
cd.say("Line 1")
cd.say("Line 2")
cd.say("Line 3")

-- 输出多个参数带换行
cd.say("Status: OK")
cd.say("Code: 200")
```

### `cd.flush(wait?)`

刷新响应输出到客户端。

参数：
- `wait`：是否等待所有数据写入（默认 false）

```lua
-- 异步刷新
cd.flush()

-- 同步刷新
cd.flush(true)
```

### `cd.eof()`

明确指定响应输出流的结束。

```lua
-- 结束响应流
cd.print("Final data")
cd.eof()
```

## 响应头操作

### `cd.header[key]`

获取或设置响应头。

```lua
-- 设置单个响应头
cd.header["Content-Type"] = "application/json"
cd.header["X-Custom-Header"] = "custom-value"

-- 设置多个相同名称的头（数组形式）
cd.header["Set-Cookie"] = {"session=abc123", "theme=dark"}

-- 获取响应头（在响应阶段无效，仅在请求阶段有效）
-- 在响应阶段，通常只设置头，不获取
```

## 响应对象

### `cd.resp`

响应对象，提供 `get_headers` 方法。

```lua
-- 获取所有响应头
local response_headers = cd.resp.get_headers()
```

## 控制流程

### `cd.exit(status)`

退出当前请求处理并返回状态码。

参数：
- `status`：HTTP 状态码（>= 200 时中断请求）

```lua
-- 正常退出并返回 200
cd.exit(200)

-- 返回 403 禁止访问
if not authorized then
    cd.status = 403
    cd.print("Access denied")
    cd.exit(403)
end

-- 返回 302 重定向
cd.header["Location"] = "https://example.com"
cd.exit(302)
```

## 时间相关

### `cd.now()`

获取当前时间戳（秒，包含毫秒小数部分）。

```lua
local current_time = cd.now()
cd.print("Current time: ", current_time)
```

### `cd.time()`

获取当前时间戳（整数秒）。

```lua
local current_time = cd.time()
cd.print("Current time (seconds): ", current_time)
```

### `cd.today()`

获取当前日期（格式：yyyy-mm-dd）。

```lua
local today = cd.today()
cd.print("Today: ", today)  -- 例如: 2023-12-25
```

### `cd.update_time()`

强制更新时间（在 Candy 中是空操作）。

```lua
-- 更新时间（仅 API 兼容性）
cd.update_time()
```

## 实际应用示例

### JSON 响应

```lua
-- 设置 JSON 响应
cd.status = 200
cd.header["Content-Type"] = "application/json"

local response = {
    status = "success",
    data = {
        message = "Hello from Candy!",
        timestamp = cd.time()
    }
}

-- 简单 JSON 序列化
local json = string.format([[{"status":"%s","data":{"message":"%s","timestamp":%d}}]],
                          response.status, response.data.message, response.data.timestamp)

cd.print(json)
```

### 重定向

```lua
-- 302 重定向
cd.status = 302
cd.header["Location"] = "https://example.com/new-location"
cd.exit(302)
```

### 文件下载

```lua
-- 设置文件下载响应
cd.status = 200
cd.header["Content-Type"] = "application/octet-stream"
cd.header["Content-Disposition"] = 'attachment; filename="document.pdf"'

-- 输出文件内容
cd.print(file_content)
```

### 流式响应

```lua
-- 流式输出数据
cd.status = 200
cd.header["Content-Type"] = "text/plain"

for i = 1, 10 do
    cd.say("Line ", i)
    cd.flush()  -- 立即发送到客户端
    
    -- 模拟延迟
    -- 注意：Candy 中没有内置的 sleep 函数，这是概念示例
end

cd.eof()
```

## 常用响应头

以下是一些常用的响应头设置：

```lua
-- JSON 响应
cd.header["Content-Type"] = "application/json"

-- HTML 响应
cd.header["Content-Type"] = "text/html; charset=utf-8"

-- JavaScript 文件
cd.header["Content-Type"] = "application/javascript"

-- CSS 文件
cd.header["Content-Type"] = "text/css"

-- 图片
cd.header["Content-Type"] = "image/png"

-- 自定义缓存控制
cd.header["Cache-Control"] = "no-cache, no-store, must-revalidate"

-- CORS 头
cd.header["Access-Control-Allow-Origin"] = "*"
cd.header["Access-Control-Allow-Methods"] = "GET, POST, PUT, DELETE"
cd.header["Access-Control-Allow-Headers"] = "Content-Type"
```