---
sidebar_label: API 参考
sidebar_position: 2
title: API 参考
---

# API 参考

本文档包含 Candy Lua 脚本的完整 API 参考，包括请求处理、响应处理、日志记录、共享字典、加密和编码函数。

## 请求 API

Candy 的 Lua 脚本提供了全面的请求处理 API，通过 `cd.req` 对象访问。这些 API 与 OpenResty 的 `cd.req.*` 系列函数兼容。

### 获取请求信息

#### `cd.req.get_method()`

获取 HTTP 请求方法（GET、POST、PUT 等）。

```lua
local method = cd.req.get_method()
cd.say("Request method: ", method)
```

#### `cd.req.get_uri()`

获取当前请求的完整 URI（包含查询参数）。

```lua
local uri = cd.req.get_uri()
cd.say("Requested URI: ", uri)
```

#### `cd.req.get_headers(max_headers?, raw?)`

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

#### `cd.req.get_uri_args(max_args?)`

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

#### `cd.req.get_post_args(max_args?)`

获取 POST 请求的表单参数（仅支持 `application/x-www-form-urlencoded`）。

```lua
local post_args = cd.req.get_post_args()

-- 访问 POST 数据：name=Jane&age=25
local name = post_args["name"]
local age = post_args["age"]

cd.say("POST Name: ", name, ", POST Age: ", age)
```

#### `cd.req.get_body_data()`

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

### 修改请求信息

#### `cd.req.set_uri(uri, jump?)`

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

#### `cd.req.set_uri_args(args)`

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

#### `cd.req.set_method(method_id)`

设置请求方法（使用预定义常量）。

```lua
-- 使用方法常量
cd.req.set_method(cd.HTTP_POST)
cd.req.set_method(cd.HTTP_GET)
cd.req.set_method(cd.HTTP_PUT)
```

### 请求体操作

#### `cd.req.read_body()`

读取请求体（在 Candy 中是空操作，因为请求体已自动读取）。

```lua
-- 在 Candy 中调用此函数不会产生实际效果
cd.req.read_body()
```

#### `cd.req.discard_body()`

丢弃当前请求体。

```lua
-- 清空请求体
cd.req.discard_body()
```

#### `cd.req.init_body(buffer_size?)`

初始化新的请求体（用于程序化构造请求体）。

参数：
- `buffer_size`：缓冲区大小（字节，默认 8KB）

```lua
-- 初始化请求体
cd.req.init_body()
```

#### `cd.req.append_body(data)`

向请求体追加数据。

```lua
-- 初始化并追加数据
cd.req.init_body()
cd.req.append_body("Hello, ")
cd.req.append_body("World!")
cd.req.finish_body()
```

#### `cd.req.finish_body()`

完成请求体写入。

```lua
-- 完成请求体写入
cd.req.finish_body()
```

### 请求头操作

#### `cd.req.set_header(name, value)`

设置请求头。

**参数**：
- `name` - 头名称
- `value` - 字符串、数组或 nil（nil 表示删除）

```lua
-- 设置单个头
cd.req.set_header("X-Custom", "value")

-- 设置多值头
cd.req.set_header("Accept", {"text/html", "application/json"})

-- 删除头
cd.req.set_header("X-Old-Header", nil)
```

#### `cd.req.clear_header(name)`

清除指定的请求头。

```lua
cd.req.clear_header("X-Custom-Header")
```

### 其他请求方法

#### `cd.req.is_internal()`

判断是否为内部请求（在 Candy 中始终返回 `false`）。

```lua
if cd.req.is_internal() then
    cd.say("This is an internal request")
else
    cd.say("This is an external request")
end
```

#### `cd.req.raw_header(no_request_line?)`

获取原始请求头字符串。

**参数**：
- `no_request_line` - 是否排除请求行（默认 false）

```lua
-- 包含请求行
local full_header = cd.req.raw_header()
-- GET /path HTTP/1.1\r\nHost: example.com\r\n...

-- 不包含请求行
local headers_only = cd.req.raw_header(true)
-- Host: example.com\r\n...
```

#### `cd.req.start_time()`

获取请求开始时间（秒，包含毫秒小数）。

```lua
local start_time = cd.req.start_time()
cd.say("Request started at: ", start_time)
```

#### `cd.req.http_version()`

获取 HTTP 版本号。

```lua
local version = cd.req.http_version()
if version then
    cd.say("HTTP Version: ", version)
else
    cd.say("Unknown HTTP version")
end
```

---

## 响应 API

Candy 的 Lua 脚本提供了全面的响应处理 API，通过 `cd` 对象和 `cd.header` 对象访问。这些 API 与 OpenResty 的 `ngx.*` 系列函数兼容。

### 设置响应状态

#### `cd.status`

设置响应的 HTTP 状态码。

```lua
-- 设置成功状态
cd.status = 200

-- 设置其他状态码
cd.status = 404  -- Not Found
cd.status = 500  -- Internal Server Error
cd.status = 302  -- Moved Temporarily
```

### 响应内容输出

#### `cd.print(...)`

输出数据到响应体，连接所有参数并发送到 HTTP 客户端。

```lua
-- 输出简单文本
cd.print("Hello, World!")

-- 输出多个参数
cd.print("User: ", "Alice", ", Age: ", 25)
```

#### `cd.say(...)`

输出数据到响应体并添加换行符。

```lua
-- 输出带换行的文本
cd.say("Line 1")
cd.say("Line 2")
cd.say("Line 3")
```

#### `cd.flush(wait?)`

刷新响应输出到客户端。

参数：
- `wait`：是否等待所有数据写入（默认 false）

```lua
-- 异步刷新
cd.flush()

-- 同步刷新
cd.flush(true)
```

#### `cd.eof()`

明确指定响应输出流的结束。

```lua
-- 结束响应流
cd.print("Final data")
cd.eof()
```

### 响应头操作

#### `cd.header[key]`

获取或设置响应头。

```lua
-- 设置单个响应头
cd.header["Content-Type"] = "application/json"
cd.header["X-Custom-Header"] = "custom-value"

-- 设置多个相同名称的头（数组形式）
cd.header["Set-Cookie"] = {"session=abc123", "theme=dark"}
```

### 响应对象

#### `cd.resp`

响应对象，提供 `get_headers` 方法。

```lua
-- 获取所有响应头
local response_headers = cd.resp.get_headers()
```

### 控制流程

#### `cd.exit(status)`

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

### 时间相关

#### `cd.now()`

获取当前时间戳（秒，包含毫秒小数部分）。

```lua
local current_time = cd.now()
cd.print("Current time: ", current_time)
```

#### `cd.time()`

获取当前时间戳（整数秒）。

```lua
local current_time = cd.time()
cd.print("Current time (seconds): ", current_time)
```

#### `cd.today()`

获取当前日期（格式：yyyy-mm-dd）。

```lua
local today = cd.today()
cd.print("Today: ", today)  -- 例如: 2023-12-25
```

#### `cd.update_time()`

强制更新时间（在 Candy 中是空操作）。

```lua
cd.update_time()
```

---

## 日志和工具函数

### 日志记录

#### `cd.log(log_level, ...)`

记录日志消息到错误日志。

参数：
- `log_level`：日志级别（使用预定义常量）
- `...`：日志消息参数

```lua
-- 使用不同日志级别
cd.log(cd.LOG_ERR, "Error occurred: ", error_msg)
cd.log(cd.LOG_WARN, "Warning: ", warning_msg)
cd.log(cd.LOG_INFO, "Info: ", info_msg)
cd.log(cd.LOG_DEBUG, "Debug: ", debug_msg)
```

#### `cd.req.log(level, ...)`

通过请求对象记录日志。

```lua
cd.req.log(cd.LOG_INFO, "Request processed")
```

### 日志级别常量

| 常量 | 值 | 说明 |
|------|-----|------|
| `cd.LOG_EMERG` | 2 | 紧急 |
| `cd.LOG_ALERT` | 4 | 警报 |
| `cd.LOG_CRIT` | 8 | 严重 |
| `cd.LOG_ERR` | 16 | 错误 |
| `cd.LOG_WARN` | 32 | 警告 |
| `cd.LOG_NOTICE` | 64 | 通知 |
| `cd.LOG_INFO` | 128 | 信息 |
| `cd.LOG_DEBUG` | 255 | 调试 |

### 工具函数

#### `cd.sleep(seconds)`

休眠指定的秒数而不阻塞。

参数：
- `seconds`：休眠秒数（支持小数，精度到毫秒）

```lua
cd.sleep(1)      -- 休眠 1 秒
cd.sleep(0.5)    -- 休眠 500 毫秒
cd.sleep(0.1)    -- 休眠 100 毫秒
```

#### `cd.escape_uri(str)` / `cd.req.escape_uri(str)`

将字符串作为 URI 组件进行转义。

```lua
local original = "hello world & special!"
local escaped = cd.escape_uri(original)
cd.print("Escaped: ", escaped)  -- hello%20world%20%26%20special%21
```

#### `cd.unescape_uri(str)` / `cd.req.unescape_uri(str)`

将 URI 编码的字符串解码。

```lua
local escaped = "hello%20world%21"
local unescaped = cd.unescape_uri(escaped)
cd.print("Unescaped: ", unescaped)  -- hello world!
```

#### `cd.encode_args(table)` / `cd.req.encode_args(table)`

将 Lua 表编码为查询参数字符串。

```lua
local args = {
    name = "John Doe",
    age = 30,
    tags = {"tech", "rust"}
}

local query_string = cd.encode_args(args)
-- name=John%20Doe&age=30&tags=tech&tags=rust
```

#### `cd.decode_args(str, max_args?)` / `cd.req.decode_args(str, max_args?)`

将查询字符串解码为 Lua 表。

```lua
local query = "name=Alice&age=25&hobby=coding&hobby=reading"
local args = cd.decode_args(query)

cd.print("Name: ", args["name"])  -- Alice
cd.print("Age: ", args["age"])    -- 25
```

### 系统信息

#### `candy` 模块

Candy 提供了一个全局的 `candy` 模块，包含系统信息：

```lua
cd.print("Candy Version: ", candy.version)
cd.print("App Name: ", candy.name)
cd.print("OS: ", candy.os)
cd.print("Architecture: ", candy.arch)
cd.print("Compiler: ", candy.compiler)
cd.print("Commit: ", candy.commit)
```

#### `candy.log(message)`

记录日志信息（使用 info 级别）。

```lua
candy.log("Application started")
candy.log("User ", user_id, " accessed the system")
```

---

## 共享字典 API

共享字典（Shared Dictionary）是 Candy 提供的跨请求数据共享机制，类似于 OpenResty 的 `cd.shared.DICT`。它允许不同请求之间共享数据，支持过期时间、LRU 淘汰等特性。

### 配置共享字典

在 `config.toml` 中配置共享字典：

```toml
# 定义共享字典
[[lua_shared_dict]]
name = "cache"
size = "10m"  # 10 MB

[[lua_shared_dict]]
name = "rate_limit"
size = "1m"   # 1 MB

[[lua_shared_dict]]
name = "sessions"
size = "5m"   # 5 MB
```

### 访问共享字典

通过 `cd.shared.dict_name` 访问已配置的共享字典：

```lua
local cache = cd.shared.cache
local rate_limit = cd.shared.rate_limit
local sessions = cd.shared.sessions
```

### 基础操作

#### `dict:get(key)`

获取键对应的值。

**返回值**：`value, flags` 或 `nil, nil`

```lua
local cache = cd.shared.cache
local value, flags = cache:get("user:123")

if value then
    cd.say("Value: ", value)
    cd.say("Flags: ", flags)
else
    cd.say("Key not found or expired")
end
```

#### `dict:get_stale(key)`

获取键对应的值（包含过期标记）。

**返回值**：`value, flags, stale` 或 `nil, nil, nil`

```lua
local cache = cd.shared.cache
local value, flags, stale = cache:get_stale("config")

if value then
    if stale then
        cd.log(cd.LOG_WARN, "Using stale data for config")
    end
    cd.say("Config: ", value)
end
```

#### `dict:set(key, value, exptime?, flags?)`

设置键值对。

**参数**：
- `key` - 键（字符串）
- `value` - 值（字符串、数字、布尔值或 nil）
- `exptime` - 过期时间（秒，可选，0 或 nil 表示永不过期）
- `flags` - 用户标志位（可选，整数）

**返回值**：`success, err, forcible`

```lua
local cache = cd.shared.cache

-- 简单设置
local ok, err, forcible = cache:set("key", "value")

-- 带过期时间（60秒）
local ok, err, forcible = cache:set("session:abc", "user_data", 60)

-- 带过期时间和标志位
local ok, err, forcible = cache:set("key", "value", 3600, 42)

-- 检查结果
if not ok then
    cd.log(cd.LOG_ERR, "Failed to set key: ", err)
elseif forcible then
    cd.log(cd.LOG_WARN, "LRU eviction occurred while setting key")
end
```

#### `dict:safe_set(key, value, exptime?, flags?)`

安全设置键值对（不淘汰现有条目）。

**返回值**：`ok, err`

```lua
local cache = cd.shared.cache
local ok, err = cache:safe_set("important_key", "important_value", 3600)

if not ok then
    if err == "no memory" then
        cd.log(cd.LOG_WARN, "Not enough memory to store key")
    end
end
```

#### `dict:add(key, value, exptime?, flags?)`

添加键值对（仅在键不存在时成功）。

**返回值**：`success, err, forcible`

```lua
local cache = cd.shared.cache
local ok, err, forcible = cache:add("unique_token", "token_value", 300)

if not ok then
    if err == "exists" then
        cd.say("Token already exists")
    end
end
```

#### `dict:safe_add(key, value, exptime?, flags?)`

安全添加键值对（不淘汰现有条目）。

**返回值**：`ok, err`

```lua
local cache = cd.shared.cache
local ok, err = cache:safe_add("new_key", "value")

if ok then
    cd.say("Added successfully")
elseif ok == false then
    cd.say("Key already exists")
else
    cd.say("No memory: ", err)
end
```

#### `dict:replace(key, value, exptime?, flags?)`

替换键值对（仅在键存在时成功）。

**返回值**：`success, err, forcible`

```lua
local cache = cd.shared.cache
local ok, err, forcible = cache:replace("existing_key", "new_value")

if not ok then
    if err == "not found" then
        cd.say("Key does not exist")
    end
end
```

#### `dict:delete(key)`

删除键。

```lua
local cache = cd.shared.cache
cache:delete("old_key")
```

### 计数器操作

#### `dict:incr(key, value, init?, init_ttl?)`

增加计数器的值。

**参数**：
- `key` - 键
- `value` - 增量（可为负数或浮点数）
- `init` - 初始值（键不存在时使用）
- `init_ttl` - 初始值的过期时间

**返回值**：`new_value, err, forcible`

```lua
local cache = cd.shared.cache

-- 简单递增
local new_val, err = cache:incr("counter", 1)

-- 带初始值的递增
local new_val, err = cache:incr("counter", 1, 0)

-- 带初始值和过期时间
local new_val, err, forcible = cache:incr("page_views", 1, 0, 86400)

if new_val then
    cd.say("New value: ", new_val)
else
    cd.say("Error: ", err)  -- "not found" 或 "not a number"
end
```

### 列表操作

#### `dict:lpush(key, value)` / `dict:rpush(key, value)`

在列表头部/尾部插入元素。

**返回值**：`length, err`

```lua
local cache = cd.shared.cache
local len, err = cache:lpush("my_list", "first_item")
local len, err = cache:rpush("my_list", "last_item")
```

#### `dict:lpop(key)` / `dict:rpop(key)`

从列表头部/尾部弹出元素。

**返回值**：`value, err`

```lua
local cache = cd.shared.cache
local value, err = cache:lpop("my_list")
local value, err = cache:rpop("my_list")

if value then
    cd.say("Popped: ", value)
end
```

#### `dict:llen(key)`

获取列表长度。

**返回值**：`length, err`（如果键不存在，返回 0）

```lua
local cache = cd.shared.cache
local len, err = cache:llen("my_list")

if len then
    cd.say("List has ", len, " items")
end
```

### 管理操作

#### `dict:get_keys(max_count?)`

获取所有键。

**参数**：
- `max_count` - 最大返回数量（默认 1024，0 表示无限制）

**返回值**：键列表（表）

```lua
local cache = cd.shared.cache

-- 获取最多 100 个键
local keys = cache:get_keys(100)

-- 获取所有键
local all_keys = cache:get_keys(0)

for i, key in ipairs(keys) do
    cd.say("Key: ", key)
end
```

#### `dict:flush_all()`

清除所有条目。

```lua
local cache = cd.shared.cache
cache:flush_all()
cd.say("Cache cleared")
```

#### `dict:flush_expired(max_count?)`

清除过期条目。

**参数**：
- `max_count` - 最大清除数量（0 或 nil 表示无限制）

**返回值**：实际清除的数量

```lua
local cache = cd.shared.cache

-- 清除所有过期条目
local count = cache:flush_expired()
cd.say("Flushed ", count, " expired entries")
```

---

## 加密和编码函数

Candy 提供了一系列加密、哈希和编码工具函数，通过 `cd.req` 对象访问。

### Base64 编码

#### `cd.req.encode_base64(str, no_padding?)`

将字符串编码为 Base64 格式。

**参数**：
- `str` - 要编码的字符串
- `no_padding` - 是否省略填充字符 `=`（可选，默认 false）

**返回值**：Base64 编码字符串

```lua
local original = "Hello, World!"
local encoded = cd.req.encode_base64(original)
cd.say("Encoded: ", encoded)  -- SGVsbG8sIFdvcmxkIQ==

-- 不带填充
local encoded_no_pad = cd.req.encode_base64(original, true)
cd.say("No padding: ", encoded_no_pad)  -- SGVsbG8sIFdvcmxkIQ
```

#### `cd.req.decode_base64(str)`

将 Base64 字符串解码为原始字节。

**参数**：
- `str` - Base64 编码的字符串

**返回值**：解码后的字符串，或 `nil`（如果输入无效）

```lua
local encoded = "SGVsbG8sIFdvcmxkIQ=="
local decoded = cd.req.decode_base64(encoded)

if decoded then
    cd.say("Decoded: ", decoded)  -- Hello, World!
else
    cd.say("Invalid base64 input")
end
```

### 哈希函数

#### `cd.req.md5(str)`

计算字符串的 MD5 哈希值（十六进制格式）。

**参数**：
- `str` - 输入字符串

**返回值**：32 字符的小写十六进制字符串

```lua
local input = "Hello, World!"
local hash = cd.req.md5(input)
cd.say("MD5: ", hash)  -- 65a8e27d8879283831b664bd8b7f0ad4
```

#### `cd.req.md5_bin(str)`

计算字符串的 MD5 哈希值（二进制格式）。

**返回值**：16 字节的二进制字符串

```lua
local input = "Hello, World!"
local hash_bin = cd.req.md5_bin(input)

-- 通常需要 Base64 编码后输出
local encoded = cd.req.encode_base64(hash_bin)
cd.say("MD5 (base64): ", encoded)
```

#### `cd.req.sha1_bin(str)`

计算字符串的 SHA-1 哈希值（二进制格式）。

**返回值**：20 字节的二进制字符串

```lua
local input = "Hello, World!"
local hash_bin = cd.req.sha1_bin(input)

-- Base64 编码输出
local encoded = cd.req.encode_base64(hash_bin)
cd.say("SHA1 (base64): ", encoded)
```

#### `cd.req.crc32_short(str)` / `cd.req.crc32_long(str)`

计算字符串的 CRC-32 校验和。

**返回值**：32 位无符号整数

```lua
local input = "Hello, World!"
local checksum = cd.req.crc32_short(input)
cd.say("CRC32: ", checksum)
```

### HMAC 函数

#### `cd.req.hmac_sha1(secret_key, str)`

计算 HMAC-SHA1 消息认证码。

**参数**：
- `secret_key` - 密钥
- `str` - 输入字符串

**返回值**：20 字节的二进制 HMAC 值

```lua
local secret = "my-secret-key"
local message = "Hello, World!"

local hmac = cd.req.hmac_sha1(secret, message)

-- 通常需要 Base64 或十六进制编码
local hmac_b64 = cd.req.encode_base64(hmac)
cd.say("HMAC-SHA1: ", hmac_b64)
```

---

## 常量

### HTTP 方法常量

| 常量 | 值 |
|------|-----|
| `cd.HTTP_GET` | 0 |
| `cd.HTTP_HEAD` | 1 |
| `cd.HTTP_PUT` | 2 |
| `cd.HTTP_POST` | 3 |
| `cd.HTTP_DELETE` | 4 |
| `cd.HTTP_OPTIONS` | 5 |
| `cd.HTTP_MKCOL` | 6 |
| `cd.HTTP_COPY` | 7 |
| `cd.HTTP_MOVE` | 8 |
| `cd.HTTP_PROPFIND` | 9 |
| `cd.HTTP_PROPPATCH` | 10 |
| `cd.HTTP_LOCK` | 11 |
| `cd.HTTP_UNLOCK` | 12 |
| `cd.HTTP_PATCH` | 13 |
| `cd.HTTP_TRACE` | 14 |

### HTTP 状态码常量

**1xx 信息响应**：
- `cd.HTTP_CONTINUE` (100)
- `cd.HTTP_SWITCHING_PROTOCOLS` (101)

**2xx 成功**：
- `cd.HTTP_OK` (200)
- `cd.HTTP_CREATED` (201)
- `cd.HTTP_ACCEPTED` (202)
- `cd.HTTP_NO_CONTENT` (204)
- `cd.HTTP_PARTIAL_CONTENT` (206)

**3xx 重定向**：
- `cd.HTTP_SPECIAL_RESPONSE` (300)
- `cd.HTTP_MOVED_PERMANENTLY` (301)
- `cd.HTTP_MOVED_TEMPORARILY` (302)
- `cd.HTTP_SEE_OTHER` (303)
- `cd.HTTP_NOT_MODIFIED` (304)
- `cd.HTTP_TEMPORARY_REDIRECT` (307)

**4xx 客户端错误**：
- `cd.HTTP_BAD_REQUEST` (400)
- `cd.HTTP_UNAUTHORIZED` (401)
- `cd.HTTP_PAYMENT_REQUIRED` (402)
- `cd.HTTP_FORBIDDEN` (403)
- `cd.HTTP_NOT_FOUND` (404)
- `cd.HTTP_NOT_ALLOWED` (405)
- `cd.HTTP_NOT_ACCEPTABLE` (406)
- `cd.HTTP_REQUEST_TIMEOUT` (408)
- `cd.HTTP_CONFLICT` (409)
- `cd.HTTP_GONE` (410)
- `cd.HTTP_UPGRADE_REQUIRED` (426)
- `cd.HTTP_TOO_MANY_REQUESTS` (429)
- `cd.HTTP_CLOSE` (444)
- `cd.HTTP_ILLEGAL` (451)

**5xx 服务器错误**：
- `cd.HTTP_INTERNAL_SERVER_ERROR` (500)
- `cd.HTTP_METHOD_NOT_IMPLEMENTED` (501)
- `cd.HTTP_BAD_GATEWAY` (502)
- `cd.HTTP_SERVICE_UNAVAILABLE` (503)
- `cd.HTTP_GATEWAY_TIMEOUT` (504)
- `cd.HTTP_VERSION_NOT_SUPPORTED` (505)
- `cd.HTTP_INSUFFICIENT_STORAGE` (507)