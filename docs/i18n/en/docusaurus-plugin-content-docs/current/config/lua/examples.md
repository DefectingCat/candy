---
sidebar_label: Practical Application Examples
sidebar_position: 5
title: Practical Application Examples
---

# Practical Application Examples

This section provides a series of practical application examples showing how to use Candy's Lua script functionality in different scenarios.

## 1. API Authentication Middleware

```lua
-- scripts/auth_middleware.lua
-- API authentication middleware example

local api_keys = {
    ["secret-key-1"] = {user_id = 1, role = "admin"},
    ["secret-key-2"] = {user_id = 2, role = "user"},
    ["secret-key-3"] = {user_id = 3, role = "user"}
}

-- Get API key from request headers
local headers = cd.req.get_headers()
local api_key = headers["x-api-key"] or headers["authorization"]

if not api_key then
    cd.status = 401
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "API key required"}]])
    cd.exit(401)
end

-- Validate API key
local user_info = api_keys[api_key]
if not user_info then
    cd.status = 401
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "Invalid API key"}]])
    cd.exit(401)
end

-- Store user information in request for subsequent processing
candy.shared.set("current_user_" .. cd.req.get_uri(), user_info.user_id)

cd.log(cd.INFO, "User authenticated: ", user_info.user_id, " (role: ", user_info.role, ")")
```

## 2. Dynamic Content Generation

```lua
-- scripts/dynamic_content.lua
-- Generate dynamic content based on request parameters

local args = cd.req.get_uri_args()
local template = args["template"] or "default"
local user_id = args["user_id"] or "guest"

-- Log request
cd.log(cd.INFO, "Generating content for user: ", user_id, " with template: ", template)

-- Select content based on template
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

## 3. Request Rate Limiting

```lua
-- scripts/rate_limit.lua
-- Simple request rate limiting implementation

local client_ip = "unknown"
local headers = cd.req.get_headers()
client_ip = headers["x-forwarded-for"] or headers["x-real-ip"] or "unknown"

-- Limit requests per minute
local window = 60  -- 60-second window
local limit = 10   -- Maximum number of requests

-- Generate client identifier
local client_key = "rate_limit:" .. client_ip
local current_time = cd.time()

-- Get current request count within the window
local request_count_str = candy.shared.get(client_key)
local request_count = tonumber(request_count_str) or 0

-- Check if limit is exceeded
if request_count >= limit then
    cd.status = 429  -- Too Many Requests
    cd.header["Content-Type"] = "application/json"
    cd.header["Retry-After"] = "60"
    cd.print([[{"error": "Rate limit exceeded", "retry_after": 60}]])
    cd.log(cd.WARN, "Rate limit exceeded for IP: ", client_ip)
    cd.exit(429)
end

-- Increment request count
request_count = request_count + 1
candy.shared.set(client_key, tostring(request_count))

-- Set expiration time
-- Note: In a real environment, you may need to periodically clean up expired counters
candy.log("Request from ", client_ip, ", count: ", request_count)

cd.log(cd.INFO, "Request allowed for IP: ", client_ip, " (count: ", request_count, ")")

-- Continue processing request
cd.status = 200
cd.header["Content-Type"] = "application/json"
cd.header["X-Rate-Limit-Remaining"] = tostring(limit - request_count)
cd.print([[{"message": "Request processed successfully", "request_number": ]] .. request_count .. [[}]])
```

## 4. Response Caching

```lua
-- scripts/cache_example.lua
-- Simple response caching implementation

local cache_key = "cache:" .. cd.req.get_uri()
local cached_response = candy.shared.get(cache_key)

-- Check if cache exists and hasn't expired
if cached_response then
    cd.log(cd.INFO, "Cache hit for: ", cd.req.get_uri())

    -- Parse cached response (simplified version, actual should use more complex serialization)
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

-- Cache miss, generate response
cd.log(cd.INFO, "Cache miss for: ", cd.req.get_uri())

-- Simulate time-consuming data retrieval
cd.sleep(0.1)  -- Simulate time-consuming operations like database queries

local response_data = {
    timestamp = cd.now(),
    uri = cd.req.get_uri(),
    method = cd.req.get_method(),
    data = "Cached response content for " .. cd.req.get_uri()
}

-- Generate response
local response_json = string.format(
    [[{"timestamp": %.3f, "uri": "%s", "method": "%s", "data": "%s"}]],
    response_data.timestamp,
    response_data.uri,
    response_data.method,
    response_data.data
)

-- Set response
cd.status = 200
cd.header["Content-Type"] = "application/json"
cd.header["X-Cache"] = "MISS"
cd.print(response_json)

-- Cache response (expires in 300 seconds)
local cache_value = tostring(200) .. "|" .. response_json
candy.shared.set(cache_key, cache_value)

cd.log(cd.INFO, "Response cached for: ", cd.req.get_uri())
```

## 5. Request Validation and Filtering

```lua
-- scripts/validation_filter.lua
-- Request validation and filtering middleware

local function validate_email(email)
    -- Simple email validation (should use stricter validation in actual applications)
    if not email then return false end
    return string.match(email, "^[%w._%-]+@[%w._%-]+$") ~= nil
end

local function validate_phone(phone)
    -- Simple phone number validation
    if not phone then return false end
    return string.match(phone, "^%d+$") ~= nil and string.len(phone) >= 10
end

-- Get POST data
local post_args = cd.req.get_post_args()
local errors = {}

-- Validate required fields
if not post_args["name"] or string.len(post_args["name"]) < 2 then
    table.insert(errors, "Name is required and must be at least 2 characters")
end

if not post_args["email"] or not validate_email(post_args["email"]) then
    table.insert(errors, "Valid email is required")
end

if post_args["phone"] and not validate_phone(post_args["phone"]) then
    table.insert(errors, "Phone number must contain only digits and be at least 10 digits long")
end

-- Check for errors
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

-- Validation passed, continue processing
cd.log(cd.INFO, "Request validation passed for user: ", post_args["name"])

-- Clean input data (prevent XSS)
local clean_name = string.gsub(post_args["name"], "[<>]", "")
local clean_email = string.gsub(post_args["email"], "[<>]", "")

-- Process valid request
cd.status = 200
cd.header["Content-Type"] = "application/json"
cd.print(string.format(
    [[{"success": true, "message": "Data processed successfully", "clean_name": "%s", "clean_email": "%s"}]],
    clean_name,
    clean_email
))
```

## 6. Dynamic Routing

```lua
-- scripts/dynamic_router.lua
-- Dynamic routing handling

local path = cd.req.get_uri()
local method = cd.req.get_method()
local args = cd.req.get_uri_args()

-- Route table
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

-- Parse path parameters
local function match_route(path, method)
    local route_handlers = routes[method]
    if not route_handlers then
        return nil
    end

    -- Direct match
    if route_handlers[path] then
        return route_handlers[path]()
    end

    -- Pattern matching (simplified version)
    if string.match(path, "^/users/%d+$") then
        local id = string.match(path, "^/users/(%d+)$")
        if routes.GET["/users/:id"] and method == "GET" then
            return routes.GET["/users/:id"](id)
        end
    end

    return nil
end

-- Execute routing
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

## 7. Response Modification Middleware

```lua
-- scripts/response_modifier.lua
-- Response modification middleware, adding CORS headers and other security headers

-- Add CORS headers
cd.header["Access-Control-Allow-Origin"] = "*"
cd.header["Access-Control-Allow-Methods"] = "GET, POST, PUT, DELETE, OPTIONS"
cd.header["Access-Control-Allow-Headers"] = "Content-Type, Authorization, X-API-Key"

-- Add security headers
cd.header["X-Content-Type-Options"] = "nosniff"
cd.header["X-Frame-Options"] = "DENY"
cd.header["X-XSS-Protection"] = "1; mode=block"
cd.header["Strict-Transport-Security"] = "max-age=31536000; includeSubDomains"

-- Add custom headers
cd.header["X-Powered-By"] = "Candy Lua Engine"
cd.header["Server"] = "Candy/" .. candy.version

-- If it's an OPTIONS request, return directly
if cd.req.get_method() == "OPTIONS" then
    cd.status = 204
    cd.exit(204)
end

-- Continue processing request
cd.log(cd.INFO, "Security headers added to response")
```

## 8. Error Handling and Recovery

```lua
-- scripts/error_handler.lua
-- Comprehensive error handling and recovery mechanism

local success, result = pcall(function()
    -- Main business logic
    local args = cd.req.get_uri_args()

    -- Simulate potentially error-prone operations
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
        -- Simulate data processing
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
    -- Error handling
    cd.log(cd.ERR, "Error in processing: ", result)

    cd.status = 500
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "Internal server error", "details": "]] .. tostring(result) .. [["}]])

    -- In production environments, detailed error information may not be exposed
    -- cd.print([[{"error": "Internal server error"}]])
else
    -- Successful processing
    cd.status = result.status
    cd.header["Content-Type"] = "application/json"
    cd.print(result.body)

    cd.log(cd.INFO, "Request processed successfully")
end
```

## 9. Database Integration Example

```lua
-- scripts/database_integration.lua
-- Database integration example (simulated)

-- Note: Candy currently does not directly support database connections
-- This is a conceptual example showing how to organize code

local db_operations = {
    -- Simulate database operations
    get_user = function(user_id)
        -- Simulate retrieving user from database
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
        -- Simulate creating user
        local new_id = math.random(1000, 9999)  -- Simulate ID generation
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
cd.print(require("cjson").encode(response))  -- Note: Requires corresponding JSON library
```

## 10. Complete API Service Example

```lua
-- scripts/full_api_service.lua
-- Complete API service example

-- Initialize application state
local app = {
    name = "Candy API Service",
    version = candy.version,
    start_time = cd.time()
}

-- Utility functions
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

-- API routing handling
local function handle_request()
    local method = cd.req.get_method()
    local uri = cd.req.get_uri()
    local args = cd.req.get_uri_args()

    -- API version routing
    if string.match(uri, "^/api/v1/") then
        -- Extract resource path
        local resource = string.match(uri, "^/api/v1/(.+)")

        if resource == "status" then
            -- Status endpoint
            local uptime = cd.time() - app.start_time
            json_response({
                status = "running",
                service = app.name,
                version = app.version,
                uptime = uptime,
                timestamp = cd.time()
            })

        elseif resource == "metrics" then
            -- Metrics endpoint
            local total_requests = tonumber(candy.shared.get("total_requests")) or 0
            json_response({
                total_requests = total_requests,
                active_connections = 1,  -- Simplified processing
                server_info = {
                    os = candy.os,
                    arch = candy.arch,
                    compiler = candy.compiler
                }
            })

        elseif string.match(resource, "^users/?") then
            -- User-related endpoints
            if method == "GET" then
                -- Get user list
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
                -- Create user
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

-- Increment request count
local current_requests = tonumber(candy.shared.get("total_requests")) or 0
candy.shared.set("total_requests", tostring(current_requests + 1))

-- Log request
cd.log(cd.INFO, "API request: ", cd.req.get_method(), " ", cd.req.get_uri())

-- Handle request
handle_request()
```

These examples demonstrate how to use Candy's Lua script functionality to implement various common web application scenarios, including authentication, rate limiting, caching, validation, routing, and more. Each example includes appropriate error handling and logging, demonstrating good practices.