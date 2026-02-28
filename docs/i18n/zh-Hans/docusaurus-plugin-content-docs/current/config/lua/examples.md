---
sidebar_label: 实际应用示例
sidebar_position: 5
title: 实际应用示例
---

# 实际应用示例

本章节提供一系列实际应用示例，展示如何在不同场景中使用 Candy 的 Lua 脚本功能。

## 1. API 认证中间件

```lua
-- scripts/auth_middleware.lua
-- API 认证中间件示例

local api_keys = {
    ["secret-key-1"] = {user_id = 1, role = "admin"},
    ["secret-key-2"] = {user_id = 2, role = "user"},
    ["secret-key-3"] = {user_id = 3, role = "user"}
}

-- 从请求头获取 API 密钥
local headers = cd.req.get_headers()
local api_key = headers["x-api-key"] or headers["authorization"]

if not api_key then
    cd.status = 401
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "API key required"}]])
    cd.exit(401)
end

-- 验证 API 密钥
local user_info = api_keys[api_key]
if not user_info then
    cd.status = 401
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "Invalid API key"}]])
    cd.exit(401)
end

-- 将用户信息存储在请求中供后续处理使用
candy.shared.set("current_user_" .. cd.req.get_uri(), user_info.user_id)

cd.log(cd.INFO, "User authenticated: ", user_info.user_id, " (role: ", user_info.role, ")")
```

## 2. 动态内容生成

```lua
-- scripts/dynamic_content.lua
-- 根据请求参数生成动态内容

local args = cd.req.get_uri_args()
local template = args["template"] or "default"
local user_id = args["user_id"] or "guest"

-- 记录请求
cd.log(cd.INFO, "Generating content for user: ", user_id, " with template: ", template)

-- 根据模板选择内容
local content
if template == "profile" then
    content = [[
    <html>
    <head><title>User Profile</title></head>
    <body>
        <h1>User Profile</h1>
        <p>User ID: ]] .. user_id .. [[</p>
        <p>Generated at: ]] .. cd.today() .. [[</p>
    </body>
    </html>
    ]]
elseif template == "dashboard" then
    content = [[
    <html>
    <head><title>Dashboard</title></head>
    <body>
        <h1>Dashboard</h1>
        <p>Welcome, User ]] .. user_id .. [[!</p>
        <p>Current time: ]] .. cd.now() .. [[</p>
    </body>
    </html>
    ]]
else
    content = [[
    <html>
    <head><title>Default Page</title></head>
    <body>
        <h1>Default Template</h1>
        <p>User: ]] .. user_id .. [[</p>
        <p>Template: ]] .. template .. [[</p>
    </body>
    </html>
    ]]
end

cd.status = 200
cd.header["Content-Type"] = "text/html; charset=utf-8"
cd.print(content)
```

## 3. 请求限流

```lua
-- scripts/rate_limit.lua
-- 简单的请求限流实现

local client_ip = "unknown"
local headers = cd.req.get_headers()
client_ip = headers["x-forwarded-for"] or headers["x-real-ip"] or "unknown"

-- 限制每分钟请求数
local window = 60  -- 60秒窗口
local limit = 10   -- 最大请求数

-- 生成客户端标识
local client_key = "rate_limit:" .. client_ip
local current_time = cd.time()

-- 获取当前窗口内的请求数
local request_count_str = candy.shared.get(client_key)
local request_count = tonumber(request_count_str) or 0

-- 检查是否超过限制
if request_count >= limit then
    cd.status = 429  -- Too Many Requests
    cd.header["Content-Type"] = "application/json"
    cd.header["Retry-After"] = "60"
    cd.print([[{"error": "Rate limit exceeded", "retry_after": 60}]])
    cd.log(cd.WARN, "Rate limit exceeded for IP: ", client_ip)
    cd.exit(429)
end

-- 增加请求数
request_count = request_count + 1
candy.shared.set(client_key, tostring(request_count))

-- 设置过期时间
-- 注意：在真实环境中，您可能需要定期清理过期的计数器
candy.log("Request from ", client_ip, ", count: ", request_count)

cd.log(cd.INFO, "Request allowed for IP: ", client_ip, " (count: ", request_count, ")")

-- 继续处理请求
cd.status = 200
cd.header["Content-Type"] = "application/json"
cd.header["X-Rate-Limit-Remaining"] = tostring(limit - request_count)
cd.print([[{"message": "Request processed successfully", "request_number": ]] .. request_count .. [[}]])
```

## 4. 响应缓存

```lua
-- scripts/cache_example.lua
-- 简单的响应缓存实现

local cache_key = "cache:" .. cd.req.get_uri()
local cached_response = candy.shared.get(cache_key)

-- 检查缓存是否存在且未过期
if cached_response then
    cd.log(cd.INFO, "Cache hit for: ", cd.req.get_uri())
    
    -- 解析缓存的响应（简化版，实际应使用更复杂的序列化）
    local parts = {}
    for part in cached_response:gmatch("[^|]+") do
        table.insert(parts, part)
    end
    
    if #parts >= 2 then
        cd.status = tonumber(parts[1]) or 200
        cd.print(parts[2])
        cd.exit(200)
    end
end

-- 缓存未命中，生成响应
cd.log(cd.INFO, "Cache miss for: ", cd.req.get_uri())

-- 模拟耗时的数据获取
cd.sleep(0.1)  -- 模拟数据库查询等耗时操作

local response_data = {
    timestamp = cd.now(),
    uri = cd.req.get_uri(),
    method = cd.req.get_method(),
    data = "Cached response content for " .. cd.req.get_uri()
}

-- 生成响应
local response_json = string.format(
    [[{"timestamp": %.3f, "uri": "%s", "method": "%s", "data": "%s"}]],
    response_data.timestamp,
    response_data.uri,
    response_data.method,
    response_data.data
)

-- 设置响应
cd.status = 200
cd.header["Content-Type"] = "application/json"
cd.header["X-Cache"] = "MISS"
cd.print(response_json)

-- 缓存响应（有效期 300 秒）
local cache_value = tostring(200) .. "|" .. response_json
candy.shared.set(cache_key, cache_value)

cd.log(cd.INFO, "Response cached for: ", cd.req.get_uri())
```

## 5. 请求验证和过滤

```lua
-- scripts/validation_filter.lua
-- 请求验证和过滤中间件

local function validate_email(email)
    -- 简单的邮箱验证（实际应用中应使用更严格的验证）
    if not email then return false end
    return string.match(email, "^[%w._%-]+@[%w._%-]+$") ~= nil
end

local function validate_phone(phone)
    -- 简单的电话号码验证
    if not phone then return false end
    return string.match(phone, "^%d+$") ~= nil and string.len(phone) >= 10
end

-- 获取 POST 数据
local post_args = cd.req.get_post_args()
local errors = {}

-- 验证必填字段
if not post_args["name"] or string.len(post_args["name"]) < 2 then
    table.insert(errors, "Name is required and must be at least 2 characters")
end

if not post_args["email"] or not validate_email(post_args["email"]) then
    table.insert(errors, "Valid email is required")
end

if post_args["phone"] and not validate_phone(post_args["phone"]) then
    table.insert(errors, "Phone number must contain only digits and be at least 10 digits long")
end

-- 检查是否有错误
if #errors > 0 then
    cd.status = 400
    cd.header["Content-Type"] = "application/json"
    
    local error_json = [[{"errors": ["]]
    for i, error in ipairs(errors) do
        if i > 1 then
            error_json = error_json .. [[, "]] .. error .. [["]]
        else
            error_json = error_json .. error .. [["]]
        end
    end
    error_json = error_json .. [[}]]
    
    cd.print(error_json)
    cd.log(cd.WARN, "Validation failed: ", error_json)
    cd.exit(400)
end

-- 验证通过，继续处理
cd.log(cd.INFO, "Request validation passed for user: ", post_args["name"])

-- 清理输入数据（防止 XSS）
local clean_name = string.gsub(post_args["name"], "[<>]", "")
local clean_email = string.gsub(post_args["email"], "[<>]", "")

-- 处理有效请求
cd.status = 200
cd.header["Content-Type"] = "application/json"
cd.print(string.format(
    [[{"success": true, "message": "Data processed successfully", "clean_name": "%s", "clean_email": "%s"}]],
    clean_name,
    clean_email
))
```

## 6. 动态路由

```lua
-- scripts/dynamic_router.lua
-- 动态路由处理

local path = cd.req.get_uri()
local method = cd.req.get_method()
local args = cd.req.get_uri_args()

-- 路由表
local routes = {
    GET = {
        ["/users"] = function()
            local page = tonumber(args["page"]) or 1
            local limit = tonumber(args["limit"]) or 10
            
            return {
                status = 200,
                body = string.format([[{"users": [], "page": %d, "limit": %d, "total": 0}]], page, limit)
            }
        end,
        
        ["/users/:id"] = function(id)
            return {
                status = 200,
                body = string.format([[{"id": %s, "name": "User %s", "email": "user%s@example.com"}]], id, id, id)
            }
        end,
        
        ["/health"] = function()
            return {
                status = 200,
                body = [[{"status": "healthy", "timestamp": "]] .. cd.time() .. [["}]]
            }
        end
    },
    
    POST = {
        ["/users"] = function()
            local post_args = cd.req.get_post_args()
            
            if not post_args["name"] or not post_args["email"] then
                return {
                    status = 400,
                    body = [[{"error": "Name and email are required"}]]
                }
            end
            
            return {
                status = 201,
                body = string.format([[{"id": 123, "name": "%s", "email": "%s", "created_at": %d}]], 
                                   post_args["name"], post_args["email"], cd.time())
            }
        end
    }
}

-- 解析路径参数
local function match_route(path, method)
    local route_handlers = routes[method]
    if not route_handlers then
        return nil
    end
    
    -- 直接匹配
    if route_handlers[path] then
        return route_handlers[path]()
    end
    
    -- 模式匹配（简化版）
    if string.match(path, "^/users/%d+$") then
        local id = string.match(path, "^/users/(%d+)$")
        if routes.GET["/users/:id"] and method == "GET" then
            return routes.GET["/users/:id"](id)
        end
    end
    
    return nil
end

-- 执行路由
local response = match_route(path, method)

if response then
    cd.status = response.status
    cd.header["Content-Type"] = "application/json"
    cd.print(response.body)
else
    cd.status = 404
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "Not found", "path": "]] .. path .. [["}]])
end
```

## 7. 响应修改中间件

```lua
-- scripts/response_modifier.lua
-- 响应修改中间件，添加 CORS 头和其他安全头

-- 添加 CORS 头
cd.header["Access-Control-Allow-Origin"] = "*"
cd.header["Access-Control-Allow-Methods"] = "GET, POST, PUT, DELETE, OPTIONS"
cd.header["Access-Control-Allow-Headers"] = "Content-Type, Authorization, X-API-Key"

-- 添加安全头
cd.header["X-Content-Type-Options"] = "nosniff"
cd.header["X-Frame-Options"] = "DENY"
cd.header["X-XSS-Protection"] = "1; mode=block"
cd.header["Strict-Transport-Security"] = "max-age=31536000; includeSubDomains"

-- 添加自定义头
cd.header["X-Powered-By"] = "Candy Lua Engine"
cd.header["Server"] = "Candy/" .. candy.version

-- 如果是 OPTIONS 请求，直接返回
if cd.req.get_method() == "OPTIONS" then
    cd.status = 204
    cd.exit(204)
end

-- 继续处理请求
cd.log(cd.INFO, "Security headers added to response")
```

## 8. 错误处理和恢复

```lua
-- scripts/error_handler.lua
-- 全面的错误处理和恢复机制

local success, result = pcall(function()
    -- 主要业务逻辑
    local args = cd.req.get_uri_args()
    
    -- 模拟可能出错的操作
    local operation = args["operation"] or "default"
    
    if operation == "divide" then
        local num1 = tonumber(args["num1"]) or 10
        local num2 = tonumber(args["num2"]) or 2
        
        if num2 == 0 then
            error("Division by zero")
        end
        
        return {
            status = 200,
            body = string.format([[{"result": %f, "operation": "division"}]], num1 / num2)
        }
    elseif operation == "process" then
        -- 模拟数据处理
        local data = args["data"] or "default data"
        local processed = string.upper(data)
        
        return {
            status = 200,
            body = string.format([[{"original": "%s", "processed": "%s"}]], data, processed)
        }
    else
        return {
            status = 400,
            body = [[{"error": "Unknown operation"}]]
        }
    end
end)

if not success then
    -- 错误处理
    cd.log(cd.ERR, "Error in processing: ", result)
    
    cd.status = 500
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "Internal server error", "details": "]] .. tostring(result) .. [["}]])
    
    -- 在生产环境中，可能不希望暴露详细的错误信息
    -- cd.print([[{"error": "Internal server error"}]])
else
    -- 成功处理
    cd.status = result.status
    cd.header["Content-Type"] = "application/json"
    cd.print(result.body)
    
    cd.log(cd.INFO, "Request processed successfully")
end
```

## 9. 数据库集成示例

```lua
-- scripts/database_integration.lua
-- 数据库集成示例（模拟）

-- 注意：Candy 目前不直接支持数据库连接
-- 这是一个概念示例，展示如何组织代码

local db_operations = {
    -- 模拟数据库操作
    get_user = function(user_id)
        -- 模拟从数据库获取用户
        if tonumber(user_id) and tonumber(user_id) > 0 then
            return {
                id = tonumber(user_id),
                name = "User " .. user_id,
                email = "user" .. user_id .. "@example.com",
                created_at = cd.time()
            }
        end
        return nil
    end,
    
    create_user = function(name, email)
        -- 模拟创建用户
        local new_id = math.random(1000, 9999)  -- 模拟生成ID
        return {
            id = new_id,
            name = name,
            email = email,
            created_at = cd.time()
        }
    end
}

local method = cd.req.get_method()
local args = cd.req.get_uri_args()

local response = {}
local status = 200

if method == "GET" then
    local user_id = args["id"]
    if user_id then
        local user = db_operations.get_user(user_id)
        if user then
            response = {user = user}
        else
            status = 404
            response = {error = "User not found"}
        end
    else
        status = 400
        response = {error = "User ID required"}
    elseif method == "POST" then
        local post_args = cd.req.get_post_args()
        if post_args["name"] and post_args["email"] then
            local new_user = db_operations.create_user(post_args["name"], post_args["email"])
            response = {user = new_user, message = "User created successfully"}
        else
            status = 400
            response = {error = "Name and email required"}
        end
    else
        status = 405
        response = {error = "Method not allowed"}
    end
end

cd.status = status
cd.header["Content-Type"] = "application/json"
cd.print(require("cjson").encode(response))  -- 注意：需要相应的 JSON 库
```

## 10. 完整的 API 服务示例

```lua
-- scripts/full_api_service.lua
-- 完整的 API 服务示例

-- 初始化应用状态
local app = {
    name = "Candy API Service",
    version = candy.version,
    start_time = cd.time()
}

-- 工具函数
local function json_response(data, status_code)
    status_code = status_code or 200
    cd.status = status_code
    cd.header["Content-Type"] = "application/json"
    cd.header["X-Response-Time"] = tostring(cd.now())
    cd.print(require("cjson").encode(data))
end

local function error_response(message, status_code)
    status_code = status_code or 400
    json_response({error = message}, status_code)
end

-- API 路由处理
local function handle_request()
    local method = cd.req.get_method()
    local uri = cd.req.get_uri()
    local args = cd.req.get_uri_args()
    
    -- API 版本路由
    if string.match(uri, "^/api/v1/") then
        -- 提取资源路径
        local resource = string.match(uri, "^/api/v1/(.+)")
        
        if resource == "status" then
            -- 状态端点
            local uptime = cd.time() - app.start_time
            json_response({
                status = "running",
                service = app.name,
                version = app.version,
                uptime = uptime,
                timestamp = cd.time()
            })
            
        elseif resource == "metrics" then
            -- 指标端点
            local total_requests = tonumber(candy.shared.get("total_requests")) or 0
            json_response({
                total_requests = total_requests,
                active_connections = 1,  -- 简化处理
                server_info = {
                    os = candy.os,
                    arch = candy.arch,
                    compiler = candy.compiler
                }
            })
            
        elseif string.match(resource, "^users/?") then
            -- 用户相关端点
            if method == "GET" then
                -- 获取用户列表
                local page = tonumber(args["page"]) or 1
                local limit = tonumber(args["limit"]) or 10
                
                json_response({
                    users = {},
                    pagination = {
                        page = page,
                        limit = limit,
                        total = 0
                    }
                })
                
            elseif method == "POST" then
                -- 创建用户
                local post_args = cd.req.get_post_args()
                if post_args["name"] and post_args["email"] then
                    json_response({
                        success = true,
                        message = "User created",
                        user = {
                            id = math.random(1000, 9999),
                            name = post_args["name"],
                            email = post_args["email"],
                            created_at = cd.time()
                        }
                    }, 201)
                else
                    error_response("Name and email are required", 400)
                end
            else
                error_response("Method not allowed", 405)
            end
        else
            error_response("Resource not found", 404)
        end
    else
        error_response("API endpoint not found", 404)
    end
end

-- 增加请求计数
local current_requests = tonumber(candy.shared.get("total_requests")) or 0
candy.shared.set("total_requests", tostring(current_requests + 1))

-- 记录请求
cd.log(cd.INFO, "API request: ", cd.req.get_method(), " ", cd.req.get_uri())

-- 处理请求
handle_request()
```

这些示例展示了如何使用 Candy 的 Lua 脚本功能实现各种常见的 Web 应用场景，包括认证、限流、缓存、验证、路由等功能。每个示例都包含了适当的错误处理和日志记录，体现了良好的实践。