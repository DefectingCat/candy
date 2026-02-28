---
sidebar_label: Lua Scripts
sidebar_position: 4
title: Lua Scripts
---

# Lua Scripts

Candy supports using Lua scripts as a route handling method, allowing you to write custom HTTP request processing logic. Candy's Lua implementation is fully compatible with OpenResty's API, enabling you to easily migrate existing scripts from Nginx + Lua environments.

## Overview

Candy's Lua script functionality provides:

- **OpenResty API Compatibility**: Supports most OpenResty APIs
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

## Document Structure

This Lua script documentation is divided into the following sections:

1. [Getting Started with Lua Scripts](./config/lua/intro.md) - Basic concepts and quick start for Lua scripts
2. [Request API](./config/lua/request-api.md) - Detailed request processing API documentation
3. [Response API](./config/lua/response-api.md) - Detailed response processing API documentation
4. [Logging and Utility Functions](./config/lua/logging-utils.md) - Logging and utility functions
5. [Practical Application Examples](./config/lua/examples.md) - Example code for various practical application scenarios
6. [Performance Optimization and Best Practices](./config/lua/performance-best-practices.md) - Performance optimization and best practices guide

## Main Features

- **OpenResty API Compatible**: Supports objects like `cd.req`, `cd.resp`, `cd.header`
- **Code Caching**: Supports Lua code caching to improve performance
- **Shared Data**: Cross-request data sharing through `candy.shared`
- **Logging System**: Integrated multi-level logging functionality
- **Request/Response Operations**: Complete request and response processing capabilities

## Limitations

- Does not support asynchronous operations
- Script execution has time limits
- Memory usage is limited
- Cannot directly access underlying system resources
- Does not support Lua C extensions