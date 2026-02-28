---
sidebar_label: Response API
sidebar_position: 3
title: Response API
---

# Response API

Candy's Lua scripts provide comprehensive response handling APIs accessible through the `cd` object and `cd.header` object. These APIs are compatible with OpenResty's `ngx.*` series of functions.

## Setting Response Status

### `cd.status`

Set the HTTP status code of the response.

```lua
-- Set success status
cd.status = 200

-- Set other status codes
cd.status = 404  -- Not Found
cd.status = 500  -- Internal Server Error
cd.status = 302  -- Moved Temporarily
```

## Response Content Output

### `cd.print(...)`

Output data to the response body, concatenating all parameters and sending to the HTTP client.

```lua
-- Output simple text
cd.print("Hello, World!")

-- Output multiple parameters
cd.print("User: ", "Alice", ", Age: ", 25)

-- Output table content
local user = {name = "Bob", age = 30}
cd.print("User: ", user.name, ", Age: ", user.age)
```

### `cd.say(...)`

Output data to the response body and add a newline character.

```lua
-- Output text with newlines
cd.say("Line 1")
cd.say("Line 2")
cd.say("Line 3")

-- Output multiple parameters with newlines
cd.say("Status: OK")
cd.say("Code: 200")
```

### `cd.flush(wait?)`

Flush response output to the client.

Parameters:
- `wait`: Whether to wait for all data to be written (default false)

```lua
-- Asynchronous flush
cd.flush()

-- Synchronous flush
cd.flush(true)
```

### `cd.eof()`

Explicitly specify the end of the response output stream.

```lua
-- End response stream
cd.print("Final data")
cd.eof()
```

## Response Header Operations

### `cd.header[key]`

Get or set response headers.

```lua
-- Set single response header
cd.header["Content-Type"] = "application/json"
cd.header["X-Custom-Header"] = "custom-value"

-- Set multiple headers with the same name (array form)
cd.header["Set-Cookie"] = {"session=abc123", "theme=dark"}

-- Get response headers (invalid in response phase, only valid in request phase)
-- In response phase, typically only set headers, not get them
```

## Response Object

### `cd.resp`

Response object, providing `get_headers` method.

```lua
-- Get all response headers
local response_headers = cd.resp.get_headers()
```

## Flow Control

### `cd.exit(status)`

Exit the current request processing and return a status code.

Parameters:
- `status`: HTTP status code (interrupts request when >= 200)

```lua
-- Exit normally and return 200
cd.exit(200)

-- Return 403 Forbidden
if not authorized then
    cd.status = 403
    cd.print("Access denied")
    cd.exit(403)
end

-- Return 302 redirect
cd.header["Location"] = "https://example.com"
cd.exit(302)
```

## Time Related

### `cd.now()`

Get the current timestamp (seconds, including fractional milliseconds).

```lua
local current_time = cd.now()
cd.print("Current time: ", current_time)
```

### `cd.time()`

Get the current timestamp (integer seconds).

```lua
local current_time = cd.time()
cd.print("Current time (seconds): ", current_time)
```

### `cd.today()`

Get the current date (format: yyyy-mm-dd).

```lua
local today = cd.today()
cd.print("Today: ", today)  -- e.g.: 2023-12-25
```

### `cd.update_time()`

Force update time (is a no-op in Candy).

```lua
-- Update time (for API compatibility only)
cd.update_time()
```

## Practical Examples

### JSON Response

```lua
-- Set JSON response
cd.status = 200
cd.header["Content-Type"] = "application/json"

local response = {
    status = "success",
    data = {
        message = "Hello from Candy!",
        timestamp = cd.time()
    }
}

-- Simple JSON serialization
local json = string.format([[{"status":"%s","data":{"message":"%s","timestamp":%d}}]],
                          response.status, response.data.message, response.data.timestamp)

cd.print(json)
```

### Redirect

```lua
-- 302 redirect
cd.status = 302
cd.header["Location"] = "https://example.com/new-location"
cd.exit(302)
```

### File Download

```lua
-- Set file download response
cd.status = 200
cd.header["Content-Type"] = "application/octet-stream"
cd.header["Content-Disposition"] = 'attachment; filename="document.pdf"'

-- Output file content
cd.print(file_content)
```

### Streaming Response

```lua
-- Stream data output
cd.status = 200
cd.header["Content-Type"] = "text/plain"

for i = 1, 10 do
    cd.say("Line ", i)
    cd.flush()  -- Send immediately to client

    -- Simulate delay
    -- Note: Candy does not have a built-in sleep function, this is a conceptual example
end

cd.eof()
```

## Common Response Headers

Here are some commonly used response header settings:

```lua
-- JSON response
cd.header["Content-Type"] = "application/json"

-- HTML response
cd.header["Content-Type"] = "text/html; charset=utf-8"

-- JavaScript file
cd.header["Content-Type"] = "application/javascript"

-- CSS file
cd.header["Content-Type"] = "text/css"

-- Image
cd.header["Content-Type"] = "image/png"

-- Custom cache control
cd.header["Cache-Control"] = "no-cache, no-store, must-revalidate"

-- CORS headers
cd.header["Access-Control-Allow-Origin"] = "*"
cd.header["Access-Control-Allow-Methods"] = "GET, POST, PUT, DELETE"
cd.header["Access-Control-Allow-Headers"] = "Content-Type"
```