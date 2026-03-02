---
sidebar_label: 共享字典 API
sidebar_position: 5
title: 共享字典 API
---

# 共享字典 API

共享字典（Shared Dictionary）是 Candy 提供的跨请求数据共享机制，类似于 OpenResty 的 `ngx.shared.DICT`。它允许不同请求之间共享数据，支持过期时间、LRU 淘汰等特性。

## 配置共享字典

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

## 访问共享字典

通过 `ngx.shared.dict_name` 访问已配置的共享字典：

```lua
local cache = ngx.shared.cache
local rate_limit = ngx.shared.rate_limit
local sessions = ngx.shared.sessions
```

## 基础操作

### `dict:get(key)`

获取键对应的值。

**返回值**：`value, flags` 或 `nil, nil`

- `value` - 存储的值（字符串）
- `flags` - 用户标志位（整数）
- 如果键不存在或已过期，返回 `nil, nil`

```lua
local cache = ngx.shared.cache
local value, flags = cache:get("user:123")

if value then
    cd.say("Value: ", value)
    cd.say("Flags: ", flags)
else
    cd.say("Key not found or expired")
end
```

### `dict:get_stale(key)`

获取键对应的值（包含过期标记）。

**返回值**：`value, flags, stale` 或 `nil, nil, nil`

- `value` - 存储的值
- `flags` - 用户标志位
- `stale` - 布尔值，表示数据是否已过期

```lua
local cache = ngx.shared.cache
local value, flags, stale = cache:get_stale("config")

if value then
    if stale then
        cd.log(cd.WARN, "Using stale data for config")
    end
    -- 可以使用过期数据作为降级方案
    cd.say("Config: ", value)
end
```

### `dict:set(key, value, exptime?, flags?)`

设置键值对。

**参数**：
- `key` - 键（字符串）
- `value` - 值（字符串、数字、布尔值或 nil）
- `exptime` - 过期时间（秒，可选，0 或 nil 表示永不过期）
- `flags` - 用户标志位（可选，整数）

**返回值**：`success, err, forcible`

- `success` - 布尔值，操作是否成功
- `err` - 错误信息字符串（成功时为 nil）
- `forcible` - 布尔值，是否触发了 LRU 淘汰

```lua
local cache = ngx.shared.cache

-- 简单设置
local ok, err, forcible = cache:set("key", "value")

-- 带过期时间（60秒）
local ok, err, forcible = cache:set("session:abc", "user_data", 60)

-- 带过期时间和标志位
local ok, err, forcible = cache:set("key", "value", 3600, 42)

-- 检查结果
if not ok then
    cd.log(cd.ERR, "Failed to set key: ", err)
elseif forcible then
    cd.log(cd.WARN, "LRU eviction occurred while setting key")
end
```

### `dict:safe_set(key, value, exptime?, flags?)`

安全设置键值对（不淘汰现有条目）。

与 `set` 不同，当内存不足时，此方法不会淘汰任何未过期的条目。

**返回值**：`ok, err`

- `ok` - 成功时返回 `true`，内存不足时返回 `nil`
- `err` - 错误信息（如 `"no memory"`）

```lua
local cache = ngx.shared.cache
local ok, err = cache:safe_set("important_key", "important_value", 3600)

if not ok then
    if err == "no memory" then
        cd.log(cd.WARN, "Not enough memory to store key")
    else
        cd.log(cd.ERR, "Error: ", err)
    end
end
```

### `dict:add(key, value, exptime?, flags?)`

添加键值对（仅在键不存在时成功）。

**返回值**：`success, err, forcible`

- 如果键已存在且未过期，`success` 为 `false`，`err` 为 `"exists"`

```lua
local cache = ngx.shared.cache
local ok, err, forcible = cache:add("unique_token", "token_value", 300)

if not ok then
    if err == "exists" then
        cd.say("Token already exists")
    else
        cd.log(cd.ERR, "Failed to add: ", err)
    end
end
```

### `dict:safe_add(key, value, exptime?, flags?)`

安全添加键值对（不淘汰现有条目）。

**返回值**：`ok, err`

- `ok` - 成功返回 `true`，键已存在返回 `false`，内存不足返回 `nil`

```lua
local cache = ngx.shared.cache
local ok, err = cache:safe_add("new_key", "value")

if ok then
    cd.say("Added successfully")
elseif ok == false then
    cd.say("Key already exists")
else
    cd.say("No memory: ", err)
end
```

### `dict:replace(key, value, exptime?, flags?)`

替换键值对（仅在键存在时成功）。

**返回值**：`success, err, forcible`

- 如果键不存在或已过期，`success` 为 `false`，`err` 为 `"not found"`

```lua
local cache = ngx.shared.cache
local ok, err, forcible = cache:replace("existing_key", "new_value")

if not ok then
    if err == "not found" then
        cd.say("Key does not exist")
    end
end
```

### `dict:delete(key)`

删除键。

```lua
local cache = ngx.shared.cache
cache:delete("old_key")
```

## 计数器操作

### `dict:incr(key, value, init?, init_ttl?)`

增加计数器的值。

**参数**：
- `key` - 键
- `value` - 增量（可为负数或浮点数）
- `init` - 初始值（键不存在时使用）
- `init_ttl` - 初始值的过期时间

**返回值**：`new_value, err, forcible`

```lua
local cache = ngx.shared.cache

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

## 列表操作

### `dict:lpush(key, value)`

在列表头部插入元素。

**返回值**：`length, err`

```lua
local cache = ngx.shared.cache
local len, err = cache:lpush("my_list", "first_item")

if len then
    cd.say("List length: ", len)
end
```

### `dict:rpush(key, value)`

在列表尾部插入元素。

**返回值**：`length, err`

```lua
local cache = ngx.shared.cache
local len, err = cache:rpush("my_list", "last_item")
```

### `dict:lpop(key)`

从列表头部弹出元素。

**返回值**：`value, err`

```lua
local cache = ngx.shared.cache
local value, err = cache:lpop("my_list")

if value then
    cd.say("Popped: ", value)
end
```

### `dict:rpop(key)`

从列表尾部弹出元素。

**返回值**：`value, err`

```lua
local cache = ngx.shared.cache
local value, err = cache:rpop("my_list")
```

### `dict:llen(key)`

获取列表长度。

**返回值**：`length, err`

- 如果键不存在，返回 0

```lua
local cache = ngx.shared.cache
local len, err = cache:llen("my_list")

if len then
    cd.say("List has ", len, " items")
elseif err then
    cd.say("Error: ", err)  -- "value not a list"
end
```

## 管理操作

### `dict:get_keys(max_count?)`

获取所有键。

**参数**：
- `max_count` - 最大返回数量（默认 1024，0 表示无限制）

**返回值**：键列表（表）

```lua
local cache = ngx.shared.cache

-- 获取最多 100 个键
local keys = cache:get_keys(100)

-- 获取所有键
local all_keys = cache:get_keys(0)

-- 默认最多 1024 个键
local default_keys = cache:get_keys()

for i, key in ipairs(keys) do
    cd.say("Key: ", key)
end
```

### `dict:flush_all()`

清除所有条目。

```lua
local cache = ngx.shared.cache
cache:flush_all()
cd.say("Cache cleared")
```

### `dict:flush_expired(max_count?)`

清除过期条目。

**参数**：
- `max_count` - 最大清除数量（0 或 nil 表示无限制）

**返回值**：实际清除的数量

```lua
local cache = ngx.shared.cache

-- 清除所有过期条目
local count = cache:flush_expired()
cd.say("Flushed ", count, " expired entries")

-- 最多清除 100 个过期条目
local count = cache:flush_expired(100)
```

## 完整示例

### 请求限流

```lua
-- scripts/rate_limit.lua
local limit = ngx.shared.rate_limit

local client_ip = cd.req.get_headers()["x-real-ip"] or "unknown"
local key = "limit:" .. client_ip
local max_requests = 100
local window = 60  -- 60秒

local count, err = limit:incr(key, 1, 0, window)

if count and count > max_requests then
    cd.status = 429
    cd.header["Content-Type"] = "application/json"
    cd.header["Retry-After"] = tostring(window)
    cd.print([[{"error": "Rate limit exceeded"}]])
    cd.exit(429)
end

cd.header["X-RateLimit-Remaining"] = tostring(max_requests - (count or 0))
```

### 会话存储

```lua
-- scripts/session.lua
local sessions = ngx.shared.sessions

local session_id = cd.req.get_headers()["x-session-id"]

if not session_id then
    -- 创建新会话
    session_id = cd.md5(cd.time() .. math.random())
    local user_data = {
        created = cd.time(),
        ip = cd.req.get_headers()["x-real-ip"]
    }
    sessions:set(session_id, user_data, 3600)  -- 1小时过期
end

-- 获取会话数据
local session_data = sessions:get(session_id)
if not session_data then
    cd.status = 401
    cd.print([[{"error": "Session expired"}]])
    cd.exit(401)
end
```

### 消息队列

```lua
-- scripts/message_queue.lua
local queue = ngx.shared.message_queue

-- 生产者：添加消息
local function produce(message)
    local len, err = queue:rpush("task_queue", message)
    if len then
        cd.say("Message queued, queue length: ", len)
    else
        cd.say("Error: ", err)
    end
end

-- 消费者：获取消息
local function consume()
    local message, err = queue:lpop("task_queue")
    if message then
        cd.say("Processing: ", message)
    else
        cd.say("No messages in queue")
    end
end

local action = cd.req.get_uri_args()["action"]
if action == "produce" then
    produce(cd.req.get_uri_args()["message"] or "default message")
else
    consume()
end
```

### 缓存层

```lua
-- scripts/cache_layer.lua
local cache = ngx.shared.cache

local cache_key = "api:" .. cd.req.get_uri()
local cached, flags = cache:get(cache_key)

if cached then
    cd.log(cd.INFO, "Cache hit for: ", cache_key)
    cd.header["X-Cache"] = "HIT"
    cd.header["Content-Type"] = "application/json"
    cd.print(cached)
    cd.exit(200)
end

-- 缓存未命中，生成数据
cd.log(cd.INFO, "Cache miss for: ", cache_key)
local response_data = generate_response()  -- 假设的生成函数

-- 缓存结果（5分钟）
cache:set(cache_key, response_data, 300)

cd.header["X-Cache"] = "MISS"
cd.header["Content-Type"] = "application/json"
cd.print(response_data)
```

## 注意事项

1. **内存限制**：共享字典有容量限制，超出时会触发 LRU 淘汰或返回错误
2. **并发安全**：所有操作都是原子性的，可以安全地在多个请求间共享
3. **数据类型**：存储的值会被转换为字符串形式
4. **过期检查**：`get` 操作会自动过滤过期条目，但过期条目不会立即删除
5. **性能建议**：避免存储过大的值，建议每个值不超过几 KB