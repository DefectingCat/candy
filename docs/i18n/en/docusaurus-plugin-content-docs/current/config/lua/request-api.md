---
sidebar_label: Request API
sidebar_position: 2
title: Request API
---

# Request API

Candy's Lua scripts provide comprehensive request handling APIs accessible through the `cd.req` object. These APIs are compatible with OpenResty's `ngx.req.*` series of functions.

## Getting Request Information

### `cd.req.get_method()`

Get the HTTP request method (GET, POST, PUT, etc.).

```lua
local method = cd.req.get_method()
cd.say("Request method: ", method)
```

### `cd.req.get_uri()`

Get the complete URI of the current request (including query parameters).

```lua
local uri = cd.req.get_uri()
cd.say("Requested URI: ", uri)
```

### `cd.req.get_headers(max_headers?, raw?)`

Get request header information.

Parameters:
- `max_headers`: Maximum number of headers to return (default 100, 0 means unlimited)
- `raw`: Whether to preserve original case (default false, converts to lowercase)

```lua
-- Get all request headers
local headers = cd.req.get_headers()

-- Get up to 50 headers
local limited_headers = cd.req.get_headers(50)

-- Get headers with original case
local raw_headers = cd.req.get_headers(0, true)

cd.say("User-Agent: ", headers["user-agent"])
cd.say("Content-Type: ", headers["content-type"])
```

### `cd.req.get_uri_args(max_args?)`

Get URI query parameters.

Parameters:
- `max_args`: Maximum number of parameters (default 100, 0 means unlimited)

```lua
local args = cd.req.get_uri_args()

-- Access ?name=John&age=30
local name = args["name"]
local age = args["age"]

cd.say("Name: ", name, ", Age: ", age)
```

### `cd.req.get_post_args(max_args?)`

Get form parameters from POST requests (only supports `application/x-www-form-urlencoded`).

```lua
local post_args = cd.req.get_post_args()

-- Access POST data: name=Jane&age=25
local name = post_args["name"]
local age = post_args["age"]

cd.say("POST Name: ", name, ", POST Age: ", age)
```

### `cd.req.get_body_data()`

Get raw request body data.

```lua
local body_data = cd.req.get_body_data()

if body_data then
    cd.say("Request body length: ", string.len(body_data))
    cd.say("Request body: ", body_data)
else
    cd.say("No request body")
end
```

## Modifying Request Information

### `cd.req.set_uri(uri, jump?)`

Set the URI for the current request.

Parameters:
- `uri`: New URI string
- `jump`: Whether to jump to the new URI (default false)

```lua
-- Change request URI
cd.req.set_uri("/new-location")

-- Change and jump
cd.req.set_uri("/redirect-target", true)
```

### `cd.req.set_uri_args(args)`

Set URI query parameters.

Parameters:
- `args`: Parameter table or query string

```lua
-- Set parameters using table
cd.req.set_uri_args({
    page = 1,
    size = 10,
    sort = "name"
})

-- Set parameters using query string
cd.req.set_uri_args("category=tech&tag=rust")
```

### `cd.req.set_method(method_id)`

Set the request method (using predefined constants).

```lua
-- Use method constants
cd.req.set_method(cd.HTTP_POST)
cd.req.set_method(cd.HTTP_GET)
cd.req.set_method(cd.HTTP_PUT)
```

## Request Body Operations

### `cd.req.read_body()`

Read the request body (is a no-op in Candy, as the request body is automatically read).

```lua
-- Calling this function in Candy has no actual effect
cd.req.read_body()
```

### `cd.req.discard_body()`

Discard the current request body.

```lua
-- Clear request body
cd.req.discard_body()
```

### `cd.req.init_body(buffer_size?)`

Initialize a new request body (for programmatic construction of request body).

Parameters:
- `buffer_size`: Buffer size (bytes, default 8KB)

```lua
-- Initialize request body
cd.req.init_body()
```

### `cd.req.append_body(data)`

Append data to the request body.

```lua
-- Initialize and append data
cd.req.init_body()
cd.req.append_body("Hello, ")
cd.req.append_body("World!")
cd.req.finish_body()
```

### `cd.req.finish_body()`

Finish writing the request body.

```lua
-- Finish writing request body
cd.req.finish_body()
```

## Time Related

### `cd.req.start_time()`

Get the request start time (seconds, including fractional milliseconds).

```lua
local start_time = cd.req.start_time()
cd.say("Request started at: ", start_time)
```

### `cd.req.http_version()`

Get the HTTP version number.

```lua
local version = cd.req.http_version()
if version then
    cd.say("HTTP Version: ", version)
else
    cd.say("Unknown HTTP version")
end
```

## Utility Functions

### `cd.req.escape_uri(str)`

Escape URI component.

```lua
local original = "hello world"
local escaped = cd.req.escape_uri(original)
cd.say("Original: ", original)
cd.say("Escaped: ", escaped)  -- hello%20world
```

### `cd.req.unescape_uri(str)`

Decode URI component.

```lua
local escaped = "hello%20world"
local unescaped = cd.req.unescape_uri(escaped)
cd.say("Escaped: ", escaped)
cd.say("Unescaped: ", unescaped)  -- hello world
```

### `cd.req.encode_args(table)`

Encode a table as a query string.

```lua
local args = {
    name = "John",
    age = 30,
    tags = {"tech", "rust"}
}

local query_string = cd.req.encode_args(args)
cd.say("Encoded: ", query_string)  -- name=John&age=30&tags=tech&tags=rust
```

### `cd.req.decode_args(str, max_args?)`

Decode a query string into a table.

```lua
local query = "name=Jane&age=25&active=true"
local args = cd.req.decode_args(query)

cd.say("Name: ", args["name"])      -- Jane
cd.say("Age: ", args["age"])        -- 25
cd.say("Active: ", args["active"])  -- true
```

## Constants

### HTTP Method Constants

- `cd.HTTP_GET` (0)
- `cd.HTTP_HEAD` (1)
- `cd.HTTP_PUT` (2)
- `cd.HTTP_POST` (3)
- `cd.HTTP_DELETE` (4)
- `cd.HTTP_OPTIONS` (5)
- `cd.HTTP_MKCOL` (6)
- `cd.HTTP_COPY` (7)
- `cd.HTTP_MOVE` (8)
- `cd.HTTP_PROPFIND` (9)
- `cd.HTTP_PROPPATCH` (10)
- `cd.HTTP_LOCK` (11)
- `cd.HTTP_UNLOCK` (12)
- `cd.HTTP_PATCH` (13)
- `cd.HTTP_TRACE` (14)

### HTTP Status Code Constants

- `cd.HTTP_OK` (200)
- `cd.HTTP_CREATED` (201)
- `cd.HTTP_NO_CONTENT` (204)
- `cd.HTTP_PARTIAL_CONTENT` (206)
- `cd.HTTP_MOVED_PERMANENTLY` (301)
- `cd.HTTP_MOVED_TEMPORARILY` (302)
- `cd.HTTP_NOT_MODIFIED` (304)
- `cd.HTTP_BAD_REQUEST` (400)
- `cd.HTTP_UNAUTHORIZED` (401)
- `cd.HTTP_FORBIDDEN` (403)
- `cd.HTTP_NOT_FOUND` (404)
- `cd.HTTP_INTERNAL_SERVER_ERROR` (500)
- `cd.HTTP_SERVICE_UNAVAILABLE` (503)