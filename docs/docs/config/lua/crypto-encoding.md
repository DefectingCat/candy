---
sidebar_label: 加密和编码函数
sidebar_position: 6
title: 加密和编码函数
---

# 加密和编码函数

Candy 提供了一系列加密、哈希和编码工具函数，通过 `cd.req` 对象访问。这些函数对于构建安全的 Web 应用非常有用。

## Base64 编码

### `cd.req.encode_base64(str, no_padding?)`

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

### `cd.req.decode_base64(str)`

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

## URI 编码

### `cd.req.escape_uri(str)`

将字符串作为 URI 组件进行转义（百分号编码）。

**参数**：
- `str` - 要转义的字符串

**返回值**：转义后的字符串

```lua
local original = "hello world & special!"
local escaped = cd.req.escape_uri(original)
cd.say("Escaped: ", escaped)  -- hello%20world%20%26%20special%21
```

### `cd.req.unescape_uri(str)`

将 URI 编码的字符串解码。

**参数**：
- `str` - URI 编码的字符串

**返回值**：解码后的字符串

```lua
local escaped = "hello%20world%21"
local unescaped = cd.req.unescape_uri(escaped)
cd.say("Unescaped: ", unescaped)  -- hello world!
```

## 查询参数编码

### `cd.req.encode_args(table)`

将 Lua 表编码为 URL 查询字符串。

**参数**：
- `table` - 键值对表

**返回值**：查询字符串

```lua
local args = {
    name = "John Doe",
    age = 30,
    tags = {"tech", "rust"},
    active = true
}

local query = cd.req.encode_args(args)
cd.say("Query: ", query)
-- name=John%20Doe&age=30&tags=tech&tags=rust&active
```

### `cd.req.decode_args(str, max_args?)`

将查询字符串解码为 Lua 表。

**参数**：
- `str` - 查询字符串
- `max_args` - 最大参数数量（默认 100，0 表示无限制）

**返回值**：参数表

```lua
local query = "name=Alice&age=25&hobby=coding&hobby=reading"
local args = cd.req.decode_args(query)

cd.say("Name: ", args["name"])  -- Alice
cd.say("Age: ", args["age"])    -- 25

-- 多值参数返回数组
if type(args["hobby"]) == "table" then
    for i, h in ipairs(args["hobby"]) do
        cd.say("Hobby ", i, ": ", h)
    end
end
```

## 哈希函数

### `cd.req.md5(str)`

计算字符串的 MD5 哈希值（十六进制格式）。

**参数**：
- `str` - 输入字符串

**返回值**：32 字符的小写十六进制字符串

```lua
local input = "Hello, World!"
local hash = cd.req.md5(input)
cd.say("MD5: ", hash)  -- 65a8e27d8879283831b664bd8b7f0ad4
```

### `cd.req.md5_bin(str)`

计算字符串的 MD5 哈希值（二进制格式）。

**参数**：
- `str` - 输入字符串

**返回值**：16 字节的二进制字符串

```lua
local input = "Hello, World!"
local hash_bin = cd.req.md5_bin(input)

-- 通常需要 Base64 编码后输出
local encoded = cd.req.encode_base64(hash_bin)
cd.say("MD5 (base64): ", encoded)
```

### `cd.req.sha1_bin(str)`

计算字符串的 SHA-1 哈希值（二进制格式）。

**参数**：
- `str` - 输入字符串

**返回值**：20 字节的二进制字符串

```lua
local input = "Hello, World!"
local hash_bin = cd.req.sha1_bin(input)

-- Base64 编码输出
local encoded = cd.req.encode_base64(hash_bin)
cd.say("SHA1 (base64): ", encoded)
```

### `cd.req.crc32_short(str)` / `cd.req.crc32_long(str)`

计算字符串的 CRC-32 校验和。

**参数**：
- `str` - 输入字符串

**返回值**：32 位无符号整数

```lua
local input = "Hello, World!"
local checksum = cd.req.crc32_short(input)
cd.say("CRC32: ", checksum)

-- crc32_long 结果相同，适用于较长输入
local checksum2 = cd.req.crc32_long(input)
cd.say("CRC32 (long): ", checksum2)
```

## HMAC 函数

### `cd.req.hmac_sha1(secret_key, str)`

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

## 日志函数

### `cd.req.log(level, ...)`

记录日志消息。

**参数**：
- `level` - 日志级别常量
- `...` - 日志消息参数

```lua
-- 不同级别的日志
cd.req.log(cd.LOG_ERR, "Error occurred: ", error_message)
cd.req.log(cd.LOG_WARN, "Warning: ", warning_message)
cd.req.log(cd.LOG_INFO, "Info: ", info_message)
cd.req.log(cd.LOG_DEBUG, "Debug: ", debug_info)
```

## 日志级别常量

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

## 实用示例

### API 签名验证

```lua
-- scripts/api_sign.lua
local function verify_signature(params, secret)
    -- 按参数名排序
    local keys = {}
    for k in pairs(params) do
        table.insert(keys, k)
    end
    table.sort(keys)
    
    -- 构建签名字符串
    local sign_str = ""
    for _, k in ipairs(keys) do
        if k ~= "sign" then
            sign_str = sign_str .. k .. "=" .. params[k] .. "&"
        end
    end
    sign_str = sign_str:sub(1, -2)  -- 移除末尾的 &
    
    -- 计算签名
    local hmac = cd.req.hmac_sha1(secret, sign_str)
    local expected_sign = cd.req.encode_base64(hmac)
    
    return expected_sign == params["sign"]
end

local args = cd.req.get_uri_args()
local secret = "your-api-secret"

if verify_signature(args, secret) then
    cd.say("Signature valid")
else
    cd.status = 401
    cd.say("Invalid signature")
end
```

### 密码哈希存储

```lua
-- scripts/password_hash.lua
local function hash_password(password, salt)
    -- 组合密码和盐
    local combined = password .. salt
    
    -- 计算哈希
    local hash = cd.req.md5(combined)
    return hash
end

local function generate_salt()
    -- 使用时间戳和随机数生成盐
    return cd.req.md5(cd.time() .. math.random()):sub(1, 16)
end

-- 创建密码
local password = "user_password_123"
local salt = generate_salt()
local hashed = hash_password(password, salt)

cd.say("Salt: ", salt)
cd.say("Hashed: ", hashed)
```

### JWT Token 生成（简化版）

```lua
-- scripts/jwt_simple.lua
local function base64url_encode(str)
    local encoded = cd.req.encode_base64(str, true)
    -- 替换 URL 不安全字符
    encoded = encoded:gsub("+", "-"):gsub("/", "_")
    return encoded
end

local function create_jwt(payload, secret)
    -- Header
    local header = '{"alg":"HS256","typ":"JWT"}'
    
    -- Payload
    local payload_json = string.format(
        '{"sub":"%s","iat":%d,"exp":%d}',
        payload.sub, payload.iat, payload.exp
    )
    
    -- 编码
    local header_b64 = base64url_encode(header)
    local payload_b64 = base64url_encode(payload_json)
    
    -- 签名
    local signing_input = header_b64 .. "." .. payload_b64
    local sig = cd.req.hmac_sha1(secret, signing_input)
    local sig_b64 = base64url_encode(sig)
    
    return signing_input .. "." .. sig_b64
end

-- 使用示例
local payload = {
    sub = "user123",
    iat = cd.time(),
    exp = cd.time() + 3600  -- 1小时后过期
}

local token = create_jwt(payload, "your-jwt-secret")
cd.say("JWT: ", token)
```

### URL 短链接

```lua
-- scripts/short_url.lua
local function generate_short_code(url)
    -- 使用 MD5 哈希生成短码
    local hash = cd.req.md5(url)
    return hash:sub(1, 8)  -- 取前8位
end

local function create_short_url(long_url)
    local short_code = generate_short_code(long_url)
    
    -- 存储映射关系
    local urls = ngx.shared.short_urls
    urls:set(short_code, long_url, 86400 * 30)  -- 30天过期
    
    return short_code
end

local long_url = cd.req.get_uri_args()["url"]
if long_url then
    local short_code = create_short_url(long_url)
    cd.say("Short URL: https://s.example.com/", short_code)
end
```

### 安全的 Token 比较

```lua
-- scripts/secure_compare.lua
local function secure_compare(a, b)
    -- 防止时序攻击的比较
    if #a ~= #b then
        return false
    end
    
    local result = 0
    for i = 1, #a do
        result = result | (string.byte(a, i) ~ string.byte(b, i))
    end
    
    return result == 0
end

-- 验证 API Token
local provided_token = cd.req.get_headers()["x-api-token"]
local expected_token = "expected-secret-token"

if secure_compare(provided_token or "", expected_token) then
    cd.say("Token valid")
else
    cd.status = 401
    cd.say("Invalid token")
end
```

## 注意事项

1. **MD5 和 SHA-1**：不推荐用于安全敏感场景，建议使用 HMAC-SHA1 或更安全的算法
2. **HMAC 密钥**：请使用足够长度和随机性的密钥
3. **Base64**：标准的 Base64 包含 `+/=` 字符，URL 安全版本需要替换这些字符
4. **时序攻击**：比较敏感数据（如 Token）时使用恒定时间比较