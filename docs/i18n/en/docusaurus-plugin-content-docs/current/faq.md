---
sidebar_label: FAQ
sidebar_position: 4
title: Frequently Asked Questions
---

## Frequently Asked Questions

This section contains answers to common questions about Candy server.

## General Questions

### What is Candy server?

Candy is a lightweight, high-performance HTTP server written in Rust. It supports static file serving, reverse proxying, Lua script handling, and HTTP redirection.

### What makes Candy different from other servers like Nginx or Apache?

Candy is designed to be simpler and more modern than traditional servers, with:
- No complex configuration syntax
- Built-in support for modern features like HTTP/2 and SSL/TLS
- Easy integration with Rust ecosystems
- High performance based on Tokio async runtime

### Is Candy production-ready?

While Candy is under active development, it is suitable for production use for many use cases. However, for high-traffic or mission-critical applications, it's recommended to test thoroughly before deployment.

## Installation and Setup

### How do I install Candy?

The recommended way to install Candy is by compiling from source:

```bash
git clone https://github.com/DefectingCat/candy.git
cd candy
cargo build --release
```

Precompiled binaries will be available in future releases.

### Can I run Candy on Windows?

Yes, Candy supports Windows 10/11 and Windows Server 2019/2022.

### How do I start Candy on boot?

For Linux systems, you can create a systemd service. For macOS, you can use launchd. For Windows, you can create a service using sc.exe or NSSM.

## Configuration

### Where is the configuration file located?

By default, Candy looks for a `config.toml` file in the current directory. You can specify a custom path using the `-c` or `--config` option.

### How do I configure virtual hosts?

Virtual hosts are configured using the `[[host]]` sections in the configuration file. Each host can have its own listen address and routes.

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"

[[host.route]]
location = "/"
root = "./html/example"
```

### How do I enable HTTPS?

Configure the SSL settings in the host section:

```toml
[[host]]
ip = "0.0.0.0"
port = 443
server_name = "example.com"
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"
```

## Performance

### How can I improve Candy's performance?

- Use release builds (`cargo build --release`)
- Adjust the number of worker threads
- Enable compression
- Use fast storage for static files
- Configure appropriate timeouts

### Does Candy support HTTP/2?

Yes, HTTP/2 support is enabled by default for HTTPS connections.

## Troubleshooting

### Candy won't start. What should I check?

1. Check that the configuration file is valid TOML
2. Verify that all specified paths (log folder, root directories, certificates) are accessible
3. Ensure the port is not already in use by another process
4. Check the log file for error messages (by default, in `logs/` directory)

### How do I enable debugging?

Set the log level to `debug` or `trace` in the configuration file:

```toml
log_level = "debug"
```

### The server is running but I can't access it from another machine. Why?

Make sure you're listening on `0.0.0.0` rather than `127.0.0.1` in your configuration:

```toml
[[host]]
ip = "0.0.0.0"  # Listen on all interfaces
port = 8080
```

Also, check your firewall settings to ensure the port is open.

## Lua Scripting

### How do I use Lua scripts with Candy?

1. Create a Lua script file (e.g., `scripts/hello.lua`)
2. Add a route in your configuration file that references the script:

```toml
[[host.route]]
location = "/hello"
lua_script = "./scripts/hello.lua"
```

3. Write your Lua code:

```lua
ctx:set_status(200)
ctx:set_header("Content-Type", "text/plain")
ctx:set_body("Hello from Lua!")
```

### What Lua API does Candy provide?

Candy provides a simple API for interacting with requests and responses through the `ctx` object:
- `ctx:get_method()` - Get HTTP method
- `ctx:get_path()` - Get request path
- `ctx:get_header(name)` - Get request header
- `ctx:set_status(status)` - Set response status
- `ctx:set_header(name, value)` - Set response header
- `ctx:set_body(body)` - Set response body

## Advanced Topics

### Can I use Candy as a reverse proxy?

Yes, use the `proxy_pass` configuration option:

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
```

### Does Candy support load balancing?

Yes, you can define upstream server groups and use load balancing methods like round robin, weighted round robin, or IP hash.

```toml
[[upstream]]
name = "backend_servers"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },
    { server = "192.168.1.101:8080", weight = 1 }
]

[[host.route]]
location = "/api"
upstream = "backend_servers"
```

### Can I customize error pages?

Yes, you can configure custom error pages using the `error_page` and `not_found_page` options.

```toml
[[host.route]]
location = "/"
root = "./html"

[host.route.error_page]
status = 500
page = "/500.html"

[host.route.not_found_page]
status = 404
page = "/404.html"
```

## Support

### Where can I get help?

- Check the [GitHub Issues](https://github.com/DefectingCat/candy/issues) page
- Create a new issue if you don't find an answer to your problem
- Contact the maintainers through the GitHub repository
