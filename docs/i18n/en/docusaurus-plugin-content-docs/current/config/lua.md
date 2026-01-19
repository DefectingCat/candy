---
sidebar_label: Lua Script
sidebar_position: 2
title: Lua Script
---

## Overview

Candy supports using Lua scripts as a routing handler, allowing you to write custom HTTP request handling logic. Lua scripts provide flexible programming capabilities that can be used with static file serving, reverse proxy, and other features without recompiling the application.

## Configuration

Add route configuration in `config.toml`:

```toml
[[host.route]]
location = "/api"
lua_script = "scripts/api_handler.lua"
```

## API Reference

### Global Variables

#### `ctx` - Request/Response Context

**Request Methods:**
- `ctx:get_path()` - Get current request path
- `ctx:get_method()` - Get HTTP request method

**Response Methods:**
- `ctx:set_status(status)` - Set response status code (default 200)
- `ctx:set_body(body)` - Set response content
- `ctx:set_header(key, value)` - Set response header

#### `candy` - Core API

**Shared Data:**
- `candy.shared.set(key, value)` - Set shared data
- `candy.shared.get(key)` - Get shared data

**Logging:**
- `candy.log(message)` - Output log message (using info level)

**System Information:**
- `candy.version` - Get version number
- `candy.name` - Get application name
- `candy.os` - Get operating system information
- `candy.arch` - Get architecture information
- `candy.compiler` - Get compiler information
- `candy.commit` - Get commit hash

## Example Scripts

### Simple Hello World

```lua
-- scripts/hello.lua
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Hello from Lua!")
```

### Accessing Shared Data

```lua
-- scripts/counter.lua
local count = candy.shared.get("page_count")
count = tonumber(count) or 0
count = count + 1
candy.shared.set("page_count", tostring(count))

ctx:set_body(string.format("Page count: %d", count))
```

### Dynamic Content Generation

```lua
-- scripts/time.lua
local time = os.date("%Y-%m-%d %H:%M:%S")
ctx:set_header("Content-Type", "text/html")
ctx:set_body(string.format([[
    <html>
    <head><title>Current Time</title></head>
    <body>
        <h1>Current Time</h1>
        <p>%s</p>
    </body>
    </html>
]], time))
```

## Best Practices

1. **Keep scripts simple** - Complex logic should be implemented in Rust. Lua is suitable for simple request/response handling.
2. **Performance considerations** - Lua script execution has overhead. Avoid complex scripts in high-concurrency scenarios.
3. **Error handling** - Include appropriate error handling in scripts.
4. **Shared data** - Use shared dictionaries appropriately to avoid resource contention.
5. **Script location** - Store Lua scripts in a separate directory (e.g., `scripts/` or `lua/`).

## Limitations

- No support for asynchronous operations
- Script execution has time limits
- Memory usage is limited
- Cannot directly access low-level system resources
- No support for Lua C extensions
