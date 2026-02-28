---
sidebar_label: Lua 脚本入门
sidebar_position: 1
title: Lua 脚本入门
---

# Lua 脚本入门

Candy 支持使用 Lua 脚本作为路由处理方式，允许您编写自定义的 HTTP 请求处理逻辑。Candy 的 Lua 实现完全兼容 OpenResty 的 API，使您可以轻松地从 Nginx + Lua 环境迁移现有脚本。

## 主要特性

- **OpenResty API 兼容**：支持大部分 OpenResty 的 API，包括 `ngx.*` 系列函数
- **高性能**：使用 mlua 库实现，提供高效的 Lua 执行环境
- **安全性**：沙箱执行环境，防止恶意脚本影响服务器
- **可扩展性**：丰富的 API 接口，满足各种复杂需求

## 配置方法

在 `config.toml` 中添加路由配置：

```toml
[[host.route]]
location = "/api"
lua_script = "scripts/api_handler.lua"
lua_code_cache = true  # 启用代码缓存以提高性能
```

## 核心概念

### 1. 请求上下文 (cd)

在 Lua 脚本中，您可以使用 `cd` 对象访问请求和响应相关信息：

- `cd.req` - 请求对象，用于获取请求信息
- `cd.resp` - 响应对象，用于设置响应信息  
- `cd.header` - 请求/响应头操作
- `cd.status` - 响应状态码

### 2. API 兼容性

Candy 实现了大量 OpenResty 的 API，包括但不限于：

- `cd.log()` - 日志记录
- `cd.print()` / `cd.say()` - 输出响应内容
- `cd.get_uri_args()` - 获取 URI 参数
- `cd.get_post_args()` - 获取 POST 参数
- `cd.get_headers()` - 获取请求头
- `cd.set_header()` - 设置响应头

## 快速开始

### 1. 简单的 Hello World

```lua
-- scripts/hello.lua
cd.say("Hello from Candy Lua!")
```

### 2. 获取请求信息

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

### 3. 设置响应

```lua
-- scripts/response_example.lua
cd.status = 200
cd.header["Content-Type"] = "application/json"

local response = '{"message": "Hello from Lua!", "status": "success"}'
cd.print(response)
```

## Lua 代码缓存

Candy 支持 Lua 代码缓存以提高性能：

- `lua_code_cache = true`：启用缓存（推荐用于生产环境）
- `lua_code_cache = false`：禁用缓存（开发期间便于调试）

当启用缓存时，Candy 会检查脚本内容的校验和，只有在脚本更改时才会重新编译。