---
sidebar_label: Getting Started with Lua Scripts
sidebar_position: 1
title: Getting Started with Lua Scripts
---

# Getting Started with Lua Scripts

Candy supports using Lua scripts as a route handling method, allowing you to write custom HTTP request processing logic. Candy's Lua implementation is fully compatible with OpenResty's API, enabling you to easily migrate existing scripts from Nginx + Lua environments.

## Main Features

- **OpenResty API Compatible**: Supports most OpenResty APIs, including `ngx.*` series functions
- **High Performance**: Implemented using the mlua library, providing an efficient Lua execution environment
- **Security**: Sandboxed execution environment to prevent malicious scripts from affecting the server
- **Extensibility**: Rich API interfaces to meet various complex requirements

## Configuration Method

Add route configuration in `config.toml`:

```toml
[[host.route]]
location = "/api"
lua_script = "scripts/api_handler.lua"
lua_code_cache = true  # Enable code caching to improve performance
```

## Core Concepts

### 1. Request Context (cd)

In Lua scripts, you can use the `cd` object to access request and response information:

- `cd.req` - Request object, used to get request information
- `cd.resp` - Response object, used to set response information
- `cd.header` - Request/response header operations
- `cd.status` - Response status code

### 2. API Compatibility

Candy implements many OpenResty APIs, including but not limited to:

- `cd.log()` - Log recording
- `cd.print()` / `cd.say()` - Output response content
- `cd.get_uri_args()` - Get URI parameters
- `cd.get_post_args()` - Get POST parameters
- `cd.get_headers()` - Get request headers
- `cd.set_header()` - Set response headers

## Quick Start

### 1. Simple Hello World

```lua
-- scripts/hello.lua
cd.say("Hello from Candy Lua!")
```

### 2. Get Request Information

```lua
-- scripts/request_info.lua
local method = cd.req.get_method()
local uri = cd.req.get_uri()
local args = cd.req.get_uri_args()

cd.say("Method: ", method)
cd.say("URI: ", uri)

for key, value in pairs(args) do
    cd.say(key, ": ", value)
end
```

### 3. Set Response

```lua
-- scripts/response_example.lua
cd.status = 200
cd.header["Content-Type"] = "application/json"

local response = '{"message": "Hello from Lua!", "status": "success"}'
cd.print(response)
```

## Lua Code Caching

Candy supports Lua code caching to improve performance:

- `lua_code_cache = true`: Enable caching (recommended for production environments)
- `lua_code_cache = false`: Disable caching (convenient for debugging during development)

When caching is enabled, Candy checks the script content checksum and only recompiles when the script changes.