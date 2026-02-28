---
sidebar_label: 性能优化与最佳实践
sidebar_position: 6
title: 性能优化与最佳实践
---

# 性能优化与最佳实践

本章节介绍如何优化 Lua 脚本的性能以及在 Candy 中使用 Lua 的最佳实践。

## 性能优化

### 1. 启用代码缓存

在生产环境中始终启用 Lua 代码缓存：

```toml
[[host.route]]
location = "/api"
lua_script = "scripts/api_handler.lua"
lua_code_cache = true  # 启用代码缓存
```

代码缓存避免了重复编译 Lua 脚本，显著提升性能。

### 2. 避免重复计算

缓存计算结果，避免在每次请求中重复计算：

```lua
-- 不好的做法：每次请求都计算
local function expensive_calculation()
    -- 耗时计算
    local result = 0
    for i = 1, 1000000 do
        result = result + i
    end
    return result
end

-- 好的做法：缓存计算结果
local cached_result = candy.shared.get("expensive_result")
if not cached_result then
    cached_result = tostring(expensive_calculation())
    candy.shared.set("expensive_result", cached_result)
end
```

### 3. 优化字符串操作

避免不必要的字符串拼接，使用高效的方式构建字符串：

```lua
-- 不好的做法：多次拼接
local response = ""
response = response .. "{"
response = response .. '"message": "'
response = response .. message
response = response .. '", '
response = response .. '"status": "success"'
response = response .. "}"

-- 好的做法：使用格式化
local response = string.format([[{"message": "%s", "status": "success"}]], message)

-- 或使用表拼接
local parts = {
    [[{"message": "]], message, [[", "status": "success"}]]
}
local response = table.concat(parts)
```

### 4. 合理使用共享数据

共享数据是跨请求的，合理使用可以提高性能，但要注意并发问题：

```lua
-- 正确使用共享数据
local counter = tonumber(candy.shared.get("request_count")) or 0
counter = counter + 1
candy.shared.set("request_count", tostring(counter))

-- 对于复杂操作，考虑原子性
local function atomic_increment(key, increment)
    local current = tonumber(candy.shared.get(key)) or 0
    candy.shared.set(key, tostring(current + increment))
    return current + increment
end
```

## 最佳实践

### 1. 错误处理

始终包含适当的错误处理：

```lua
local success, result = pcall(function()
    -- 业务逻辑
    local data = risky_operation()
    return data
end)

if not success then
    cd.log(cd.ERR, "Operation failed: ", result)
    cd.status = 500
    cd.print([[{"error": "Internal server error"}]])
    return
end

-- 成功处理
cd.print(result)
```

### 2. 资源清理

及时清理不再需要的资源：

```lua
-- 清理临时数据
local temp_key = "temp_data_" .. cd.time()
candy.shared.set(temp_key, "temporary data")

-- 在适当的时候清理
-- （实际应用中可能需要定时清理机制）
```

### 3. 输入验证

始终验证外部输入：

```lua
local args = cd.req.get_uri_args()

-- 验证参数存在性
if not args["user_id"] then
    cd.status = 400
    cd.print([[{"error": "user_id is required"}]])
    return
end

-- 验证参数类型和范围
local user_id = tonumber(args["user_id"])
if not user_id or user_id <= 0 or user_id > 999999 then
    cd.status = 400
    cd.print([[{"error": "Invalid user_id"}]])
    return
end
```

### 4. 安全考虑

防止常见的安全漏洞：

```lua
-- 防止 XSS
local function sanitize_output(str)
    if not str then return "" end
    str = string.gsub(str, "[<>\"']", function(char)
        return {
            ["<"] = "&lt;",
            [">"] = "&gt;",
            ['"'] = "&quot;",
            ["'"] = "&#x27;"
        }[char] or char
    end)
    return str
end

local user_input = args["input"] or ""
local safe_output = sanitize_output(user_input)
cd.print(safe_output)
```

### 5. 日志记录

使用适当的日志级别：

```lua
-- 调试信息
cd.log(cd.DEBUG, "Processing request for user: ", user_id)

-- 一般信息
cd.log(cd.INFO, "User logged in: ", user_id)

-- 警告
cd.log(cd.WARN, "Deprecated API endpoint accessed")

-- 错误
cd.log(cd.ERR, "Database connection failed: ", error_message)
```

### 6. 避免阻塞操作

避免在 Lua 脚本中执行长时间运行的操作：

```lua
-- 不好的做法：阻塞操作
for i = 1, 10000000 do
    -- 长时间循环
end

-- 好的做法：异步处理或委托给后端服务
-- Candy 的 Lua 环境不支持真正的异步，所以应避免长时间操作
```

## 性能监控

### 1. 响应时间监控

```lua
local start_time = cd.now()

-- 执行主要逻辑
local result = process_request()

local end_time = cd.now()
local response_time = end_time - start_time

-- 记录慢请求
if response_time > 1.0 then  -- 1秒以上
    cd.log(cd.WARN, "Slow request detected: ", response_time, " seconds")
end

-- 添加响应时间头
cd.header["X-Response-Time"] = string.format("%.3f", response_time)
```

### 2. 资源使用监控

```lua
-- 监控请求频率
local req_count = tonumber(candy.shared.get("req_per_minute")) or 0
req_count = req_count + 1
candy.shared.set("req_per_minute", tostring(req_count))

-- 每分钟重置（需要定时任务）
local current_minute = math.floor(cd.time() / 60)
local last_reset = tonumber(candy.shared.get("last_reset_minute")) or 0

if current_minute > last_reset then
    candy.shared.set("req_per_minute", "0")
    candy.shared.set("last_reset_minute", tostring(current_minute))
end
```

## 代码组织

### 1. 模块化设计

将公共功能提取到独立的函数或模块：

```lua
-- 公共工具函数
local utils = {
    validate_email = function(email)
        return string.match(email, "^[%w._%-]+@[%w._%-]+$") ~= nil
    end,
    
    sanitize_input = function(input)
        if not input then return "" end
        return string.gsub(input, "[<>\"']", "")
    end,
    
    build_response = function(data, status)
        status = status or 200
        cd.status = status
        cd.header["Content-Type"] = "application/json"
        cd.print(require("cjson").encode(data))
    end
}

-- 在主逻辑中使用
local email = utils.sanitize_input(args["email"])
if utils.validate_email(email) then
    -- 处理有效邮箱
    utils.build_response({success = true, email = email})
else
    utils.build_response({error = "Invalid email"}, 400)
end
```

### 2. 配置管理

将配置参数外部化：

```lua
-- 使用共享数据存储配置
local config = {
    rate_limit = tonumber(candy.shared.get("rate_limit")) or 100,
    cache_ttl = tonumber(candy.shared.get("cache_ttl")) or 300,
    debug_mode = candy.shared.get("debug_mode") == "true"
}

-- 根据配置调整行为
if config.debug_mode then
    cd.log(cd.DEBUG, "Debug mode enabled")
end
```

## 调试技巧

### 1. 调试信息

```lua
-- 在开发时添加调试信息
local debug_mode = args["debug"] == "true" or false

if debug_mode then
    cd.log(cd.DEBUG, "Method: ", cd.req.get_method())
    cd.log(cd.DEBUG, "URI: ", cd.req.get_uri())
    cd.log(cd.DEBUG, "Headers: ", require("cjson").encode(cd.req.get_headers()))
end
```

### 2. 性能分析

```lua
-- 性能分析函数
local function profile_function(func, ...)
    local start = cd.now()
    local result = func(...)
    local elapsed = cd.now() - start
    cd.log(cd.DEBUG, "Function took ", elapsed, " seconds")
    return result
end

-- 使用性能分析
local data = profile_function(expensive_operation)
```

遵循这些最佳实践可以帮助您构建高性能、安全可靠的 Lua 脚本，充分利用 Candy 的强大功能。