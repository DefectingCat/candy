---
sidebar_label: 请求 API
sidebar_position: 2
title: 请求 API
---

# 请求 API

Candy 的 Lua 脚本提供了全面的请求处理 API，通过 `cd.req` 对象访问。这些 API 与 OpenResty 的 `ngx.req.*` 系列函数兼容。

## 获取请求信息

### `cd.req.get_method()`

获取 HTTP 请求方法（GET、POST、PUT 等）。

```lua
local method = cd.req.get_method()
cd.say("Request method: ", method)
```

### `cd.req.get_uri()`

获取当前请求的完整 URI（包含查询参数）。

```lua
local uri = cd.req.get_uri()
cd.say("Requested URI: ", uri)
```

### `cd.req.get_headers(max_headers?, raw?)`

获取请求头信息。

参数：
- `max_headers`：最大返回头数量（默认 100，0 表示无限制）
- `raw`：是否保持原始大小写（默认 false，会转换为小写）

```lua
-- 获取所有请求头
local headers = cd.req.get_headers()

-- 获取最多 50 个头
local limited_headers = cd.req.get_headers(50)

-- 获取原始大小写的头
local raw_headers = cd.req.get_headers(0, true)

cd.say("User-Agent: ", headers["user-agent"])
cd.say("Content-Type: ", headers["content-type"])
```

### `cd.req.get_uri_args(max_args?)`

获取 URI 查询参数。

参数：
- `max_args`：最大参数数量（默认 100，0 表示无限制）

```lua
local args = cd.req.get_uri_args()

-- 访问 ?name=John&age=30
local name = args["name"]
local age = args["age"]

cd.say("Name: ", name, ", Age: ", age)
```

### `cd.req.get_post_args(max_args?)`

获取 POST 请求的表单参数（仅支持 `application/x-www-form-urlencoded`）。

```lua
local post_args = cd.req.get_post_args()

-- 访问 POST 数据：name=Jane&age=25
local name = post_args["name"]
local age = post_args["age"]

cd.say("POST Name: ", name, ", POST Age: ", age)
```

### `cd.req.get_body_data()`

获取原始请求体数据。

```lua
local body_data = cd.req.get_body_data()

if body_data then
    cd.say("Request body length: ", string.len(body_data))
    cd.say("Request body: ", body_data)
else
    cd.say("No request body")
end
```

## 修改请求信息

### `cd.req.set_uri(uri, jump?)`

设置当前请求的 URI。

参数：
- `uri`：新的 URI 字符串
- `jump`：是否跳转到新 URI（默认 false）

```lua
-- 更改请求 URI
cd.req.set_uri("/new-location")

-- 更改并跳转
cd.req.set_uri("/redirect-target", true)
```

### `cd.req.set_uri_args(args)`

设置 URI 查询参数。

参数：
- `args`：参数表或查询字符串

```lua
-- 使用表设置参数
cd.req.set_uri_args({
    page = 1,
    size = 10,
    sort = "name"
})

-- 使用查询字符串设置参数
cd.req.set_uri_args("category=tech&tag=rust")
```

### `cd.req.set_method(method_id)`

设置请求方法（使用预定义常量）。

```lua
-- 使用方法常量
cd.req.set_method(cd.HTTP_POST)
cd.req.set_method(cd.HTTP_GET)
cd.req.set_method(cd.HTTP_PUT)
```

## 请求体操作

### `cd.req.read_body()`

读取请求体（在 Candy 中是空操作，因为请求体已自动读取）。

```lua
-- 在 Candy 中调用此函数不会产生实际效果
cd.req.read_body()
```

### `cd.req.discard_body()`

丢弃当前请求体。

```lua
-- 清空请求体
cd.req.discard_body()
```

### `cd.req.init_body(buffer_size?)`

初始化新的请求体（用于程序化构造请求体）。

参数：
- `buffer_size`：缓冲区大小（字节，默认 8KB）

```lua
-- 初始化请求体
cd.req.init_body()
```

### `cd.req.append_body(data)`

向请求体追加数据。

```lua
-- 初始化并追加数据
cd.req.init_body()
cd.req.append_body("Hello, ")
cd.req.append_body("World!")
cd.req.finish_body()
```

### `cd.req.finish_body()`

完成请求体写入。

```lua
-- 完成请求体写入
cd.req.finish_body()
```

## 时间相关

### `cd.req.start_time()`

获取请求开始时间（秒，包含毫秒小数）。

```lua
local start_time = cd.req.start_time()
cd.say("Request started at: ", start_time)
```

### `cd.req.http_version()`

获取 HTTP 版本号。

```lua
local version = cd.req.http_version()
if version then
    cd.say("HTTP Version: ", version)
else
    cd.say("Unknown HTTP version")
end
```

## 实用工具函数

### `cd.req.escape_uri(str)`

转义 URI 组件。

```lua
local original = "hello world"
local escaped = cd.req.escape_uri(original)
cd.say("Original: ", original)
cd.say("Escaped: ", escaped)  -- hello%20world
```

### `cd.req.unescape_uri(str)`

解码 URI 组件。

```lua
local escaped = "hello%20world"
local unescaped = cd.req.unescape_uri(escaped)
cd.say("Escaped: ", escaped)
cd.say("Unescaped: ", unescaped)  -- hello world
```

### `cd.req.encode_args(table)`

将表编码为查询字符串。

```lua
local args = {
    name = "John",
    age = 30,
    tags = {"tech", "rust"}
}

local query_string = cd.req.encode_args(args)
cd.say("Encoded: ", query_string)  -- name=John&age=30&tags=tech&tags=rust
```

### `cd.req.decode_args(str, max_args?)`

将查询字符串解码为表。

```lua
local query = "name=Jane&age=25&active=true"
local args = cd.req.decode_args(query)

cd.say("Name: ", args["name"])      -- Jane
cd.say("Age: ", args["age"])        -- 25
cd.say("Active: ", args["active"])  -- true
```

## 常量

### HTTP 方法常量

- `cd.HTTP_GET` (0)
- `cd.HTTP_HEAD` (1) 
- `cd.HTTP_PUT` (2)
- `cd.HTTP_POST` (3)
- `cd.HTTP_DELETE` (4)
- `cd.HTTP_OPTIONS` (5)
- `cd.HTTP_MKCOL` (6)
- `cd.HTTP_COPY` (7)
- `cd.HTTP_MOVE` (8)
- `cd.HTTP_PROPFIND` (9)
- `cd.HTTP_PROPPATCH` (10)
- `cd.HTTP_LOCK` (11)
- `cd.HTTP_UNLOCK` (12)
- `cd.HTTP_PATCH` (13)
- `cd.HTTP_TRACE` (14)

### HTTP 状态码常量

- `cd.HTTP_OK` (200)
- `cd.HTTP_CREATED` (201)
- `cd.HTTP_NO_CONTENT` (204)
- `cd.HTTP_PARTIAL_CONTENT` (206)
- `cd.HTTP_MOVED_PERMANENTLY` (301)
- `cd.HTTP_MOVED_TEMPORARILY` (302)
- `cd.HTTP_NOT_MODIFIED` (304)
- `cd.HTTP_BAD_REQUEST` (400)
- `cd.HTTP_UNAUTHORIZED` (401)
- `cd.HTTP_FORBIDDEN` (403)
- `cd.HTTP_NOT_FOUND` (404)
- `cd.HTTP_INTERNAL_SERVER_ERROR` (500)
- `cd.HTTP_SERVICE_UNAVAILABLE` (503)