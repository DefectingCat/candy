---
sidebar_label: Logging and Utility Functions
sidebar_position: 4
title: Logging and Utility Functions
---

# Logging and Utility Functions

Candy's Lua scripts provide rich logging and utility functions to help developers debug and monitor script execution.

## Logging

### `cd.log(log_level, ...)`

Record log messages to the error log.

Parameters:
- `log_level`: Log level (using predefined constants)
- `...`: Log message arguments

```lua
-- Using different log levels
cd.log(cd.ERR, "Error occurred: ", error_msg)
cd.log(cd.WARN, "Warning: ", warning_msg)
cd.log(cd.INFO, "Info: ", info_msg)
cd.log(cd.DEBUG, "Debug: ", debug_msg)

-- Recording multiple arguments
local user_id = 123
local action = "login"
cd.log(cd.INFO, "User ", user_id, " performed action: ", action)
```

### Log Level Constants

- `cd.EMERG` (2) - Emergency
- `cd.ALERT` (4) - Alert
- `cd.CRIT` (8) - Critical
- `cd.ERR` (16) - Error
- `cd.WARN` (32) - Warning
- `cd.NOTICE` (64) - Notice
- `cd.INFO` (128) - Info
- `cd.DEBUG` (255) - Debug

## Utility Functions

### `cd.sleep(seconds)`

Sleep for a specified number of seconds without blocking.

Parameters:
- `seconds`: Number of seconds to sleep (supports decimals, precision to milliseconds)

```lua
-- Sleep for 1 second
cd.sleep(1)

-- Sleep for 0.5 seconds (500 milliseconds)
cd.sleep(0.5)

-- Sleep for 100 milliseconds
cd.sleep(0.1)
```

### `cd.escape_uri(str)`

Escape a string as a URI component.

```lua
local original = "hello world & special chars!"
local escaped = cd.escape_uri(original)
cd.print("Original: ", original)
cd.print("Escaped: ", escaped)  -- hello%20world%20%26%20special%20chars%21
```

### `cd.unescape_uri(str)`

Decode an escaped URI component string.

```lua
local escaped = "hello%20world%21"
local unescaped = cd.unescape_uri(escaped)
cd.print("Escaped: ", escaped)
cd.print("Unescaped: ", unescaped)  -- hello world!
```

### `cd.encode_args(table)`

Encode a Lua table as a query parameter string.

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

Decode a URI-encoded query string into a Lua table.

Parameters:
- `str`: Query string
- `max_args`: Maximum number of arguments (default 100, 0 means unlimited)

```lua
local query = "name=Alice&age=25&hobby=coding&hobby=reading"
local args = cd.decode_args(query)

cd.print("Name: ", args["name"])  -- Alice
cd.print("Age: ", args["age"])    -- 25

-- Multi-value parameters
for i, hobby in ipairs(args["hobby"] or {}) do
    cd.print("Hobby ", i, ": ", hobby)
end
```

## System Information

### `candy` Module

Candy provides a global `candy` module containing system information:

```lua
-- Get system information
cd.print("Candy Version: ", candy.version)
cd.print("App Name: ", candy.name)
cd.print("OS: ", candy.os)
cd.print("Architecture: ", candy.arch)
cd.print("Compiler: ", candy.compiler)
cd.print("Commit: ", candy.commit)

-- Use candy.log to record logs
candy.log("Application started")
candy.log("Running on ", candy.os, "/", candy.arch)
```

### `candy.log(message)`

Record log information (using info level).

```lua
-- Record simple information
candy.log("Request processed")

-- Record information with variables
local user_id = 123
candy.log("User ", user_id, " accessed the system")
```

### `candy.shared`

Shared data storage for sharing data between requests.

```lua
-- Set shared data
candy.shared.set("counter", "100")

-- Get shared data
local counter = candy.shared.get("counter")
cd.print("Counter: ", counter)

-- Counter increment example
local current_count = tonumber(candy.shared.get("request_counter")) or 0
current_count = current_count + 1
candy.shared.set("request_counter", tostring(current_count))

cd.print("Request number: ", current_count)
```

## Time Functions

### `cd.now()`

Get the current timestamp (seconds, including fractional milliseconds).

```lua
local start_time = cd.now()

-- Perform some operations
-- ...

local end_time = cd.now()
local elapsed = end_time - start_time
cd.log(cd.INFO, "Operation took ", elapsed, " seconds")
```

### `cd.time()`

Get the current timestamp (integer seconds).

```lua
local current_timestamp = cd.time()
cd.print("Current timestamp: ", current_timestamp)
```

### `cd.today()`

Get the current date (format: yyyy-mm-dd).

```lua
local today = cd.today()
cd.print("Today is: ", today)  -- e.g.: 2023-12-25
```

## Utility Examples

### Request Counter

```lua
-- Implement a simple request counter
local counter = tonumber(candy.shared.get("total_requests")) or 0
counter = counter + 1
candy.shared.set("total_requests", tostring(counter))

cd.print("Total requests served: ", counter)
```

### Response Time Measurement

```lua
local start_time = cd.now()

-- Execute main logic
local result = process_request()

local end_time = cd.now()
local response_time = end_time - start_time

-- Record performance metrics
candy.log("Request processed in ", response_time, " seconds")

-- Include performance information in response (optional)
cd.header["X-Response-Time"] = tostring(response_time)
cd.print(result)
```

### User Activity Tracking

```lua
-- Track user activity
local user_id = get_user_id()  -- Assume this is a function to get user ID
local last_activity = candy.shared.get("user_" .. user_id .. "_last_active")
candy.shared.set("user_" .. user_id .. "_last_active", tostring(cd.time()))

cd.print("User ", user_id, " activity recorded")
```

### Debug Information Collection

```lua
-- Collect debug information
cd.log(cd.DEBUG, "Request method: ", cd.req.get_method())
cd.log(cd.DEBUG, "Request URI: ", cd.req.get_uri())

local headers = cd.req.get_headers()
for name, value in pairs(headers) do
    cd.log(cd.DEBUG, "Header: ", name, " = ", value)
end

-- Conditional debug output
if debug_mode then
    local body_data = cd.req.get_body_data()
    if body_data then
        cd.log(cd.DEBUG, "Request body length: ", string.len(body_data))
    end
end
```

## Error Handling and Logging

```lua
-- Complete error handling example
local success, result = pcall(function()
    -- Attempt to execute potentially failing operation
    local data = risky_operation()
    return data
end)

if not success then
    -- Log error
    cd.log(cd.ERR, "Operation failed: ", result)

    -- Set error response
    cd.status = 500
    cd.header["Content-Type"] = "application/json"
    cd.print([[{"error": "Internal server error", "message": "]] .. result .. [["}]])
else
    -- Operation succeeded, record information
    cd.log(cd.INFO, "Operation completed successfully")
    cd.print(result)
end
```

These utility functions provide your Lua scripts with powerful logging, debugging, and utility capabilities, helping you build robust and maintainable applications.