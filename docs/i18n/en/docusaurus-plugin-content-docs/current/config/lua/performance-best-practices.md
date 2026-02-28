---
sidebar_label: Performance Optimization and Best Practices
sidebar_position: 6
title: Performance Optimization and Best Practices
---

# Performance Optimization and Best Practices

This chapter introduces how to optimize the performance of Lua scripts and best practices for using Lua in Candy.

## Performance Optimization

### 1. Enable Code Caching

Always enable Lua code caching in production environments:

```toml
[[host.route]]
location = "/api"
lua_script = "scripts/api_handler.lua"
lua_code_cache = true  # Enable code caching
```

Code caching avoids repeatedly compiling Lua scripts, significantly improving performance.

### 2. Avoid Repetitive Calculations

Cache calculation results to avoid repetitive calculations on each request:

```lua
-- Bad practice: Calculate on every request
local function expensive_calculation()
    -- Time-consuming calculation
    local result = 0
    for i = 1, 1000000 do
        result = result + i
    end
    return result
end

-- Good practice: Cache calculation results
local cached_result = candy.shared.get("expensive_result")
if not cached_result then
    cached_result = tostring(expensive_calculation())
    candy.shared.set("expensive_result", cached_result)
end
```

### 3. Optimize String Operations

Avoid unnecessary string concatenation, use efficient ways to build strings:

```lua
-- Bad practice: Multiple concatenations
local response = ""
response = response .. "{"
response = response .. '"message": "'
response = response .. message
response = response .. '", '
response = response .. '"status": "success"'
response = response .. "}"

-- Good practice: Use formatting
local response = string.format([[{"message": "%s", "status": "success"}]], message)

-- Or use table concatenation
local parts = {
    [[{"message": "]], message, [[", "status": "success"}]]
}
local response = table.concat(parts)
```

### 4. Properly Use Shared Data

Shared data is cross-request, proper use can improve performance, but pay attention to concurrency issues:

```lua
-- Correct use of shared data
local counter = tonumber(candy.shared.get("request_count")) or 0
counter = counter + 1
candy.shared.set("request_count", tostring(counter))

-- For complex operations, consider atomicity
local function atomic_increment(key, increment)
    local current = tonumber(candy.shared.get(key)) or 0
    candy.shared.set(key, tostring(current + increment))
    return current + increment
end
```

## Best Practices

### 1. Error Handling

Always include appropriate error handling:

```lua
local success, result = pcall(function()
    -- Business logic
    local data = risky_operation()
    return data
end)

if not success then
    cd.log(cd.ERR, "Operation failed: ", result)
    cd.status = 500
    cd.print([[{"error": "Internal server error"}]])
    return
end

-- Successful processing
cd.print(result)
```

### 2. Resource Cleanup

Clean up resources that are no longer needed in a timely manner:

```lua
-- Clean up temporary data
local temp_key = "temp_data_" .. cd.time()
candy.shared.set(temp_key, "temporary data")

-- Clean up at appropriate times
-- (Real applications may need scheduled cleanup mechanisms)
```

### 3. Input Validation

Always validate external input:

```lua
local args = cd.req.get_uri_args()

-- Validate parameter existence
if not args["user_id"] then
    cd.status = 400
    cd.print([[{"error": "user_id is required"}]])
    return
end

-- Validate parameter type and range
local user_id = tonumber(args["user_id"])
if not user_id or user_id <= 0 or user_id > 999999 then
    cd.status = 400
    cd.print([[{"error": "Invalid user_id"}]])
    return
end
```

### 4. Security Considerations

Prevent common security vulnerabilities:

```lua
-- Prevent XSS
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

### 5. Logging

Use appropriate log levels:

```lua
-- Debug information
cd.log(cd.DEBUG, "Processing request for user: ", user_id)

-- General information
cd.log(cd.INFO, "User logged in: ", user_id)

-- Warnings
cd.log(cd.WARN, "Deprecated API endpoint accessed")

-- Errors
cd.log(cd.ERR, "Database connection failed: ", error_message)
```

### 6. Avoid Blocking Operations

Avoid performing long-running operations in Lua scripts:

```lua
-- Bad practice: Blocking operations
for i = 1, 10000000 do
    -- Long-running loop
end

-- Good practice: Async processing or delegating to backend services
-- Candy's Lua environment does not support true async, so avoid long operations
```

## Performance Monitoring

### 1. Response Time Monitoring

```lua
local start_time = cd.now()

-- Execute main logic
local result = process_request()

local end_time = cd.now()
local response_time = end_time - start_time

-- Record slow requests
if response_time > 1.0 then  -- Over 1 second
    cd.log(cd.WARN, "Slow request detected: ", response_time, " seconds")
end

-- Add response time header
cd.header["X-Response-Time"] = string.format("%.3f", response_time)
```

### 2. Resource Usage Monitoring

```lua
-- Monitor request frequency
local req_count = tonumber(candy.shared.get("req_per_minute")) or 0
req_count = req_count + 1
candy.shared.set("req_per_minute", tostring(req_count))

-- Reset every minute (requires scheduled task)
local current_minute = math.floor(cd.time() / 60)
local last_reset = tonumber(candy.shared.get("last_reset_minute")) or 0

if current_minute > last_reset then
    candy.shared.set("req_per_minute", "0")
    candy.shared.set("last_reset_minute", tostring(current_minute))
end
```

## Code Organization

### 1. Modular Design

Extract common functionality into independent functions or modules:

```lua
-- Common utility functions
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

-- Use in main logic
local email = utils.sanitize_input(args["email"])
if utils.validate_email(email) then
    -- Process valid email
    utils.build_response({success = true, email = email})
else
    utils.build_response({error = "Invalid email"}, 400)
end
```

### 2. Configuration Management

Externalize configuration parameters:

```lua
-- Use shared data to store configuration
local config = {
    rate_limit = tonumber(candy.shared.get("rate_limit")) or 100,
    cache_ttl = tonumber(candy.shared.get("cache_ttl")) or 300,
    debug_mode = candy.shared.get("debug_mode") == "true"
}

-- Adjust behavior based on configuration
if config.debug_mode then
    cd.log(cd.DEBUG, "Debug mode enabled")
end
```

## Debugging Tips

### 1. Debug Information

```lua
-- Add debug information during development
local debug_mode = args["debug"] == "true" or false

if debug_mode then
    cd.log(cd.DEBUG, "Method: ", cd.req.get_method())
    cd.log(cd.DEBUG, "URI: ", cd.req.get_uri())
    cd.log(cd.DEBUG, "Headers: ", require("cjson").encode(cd.req.get_headers()))
end
```

### 2. Performance Profiling

```lua
-- Performance profiling function
local function profile_function(func, ...)
    local start = cd.now()
    local result = func(...)
    local elapsed = cd.now() - start
    cd.log(cd.DEBUG, "Function took ", elapsed, " seconds")
    return result
end

-- Use performance profiling
local data = profile_function(expensive_operation)
```

Following these best practices can help you build high-performance, secure and reliable Lua scripts, fully leveraging Candy's powerful features.