---
sidebar_label: FAQ
sidebar_position: 5
title: Frequently Asked Questions and Troubleshooting
---

## Frequently Asked Questions

### 1. How do I install Candy?

**Method 1: Build from Source**

```bash
git clone https://github.com/DefectingCat/candy.git
cd candy
cargo build --release
```

**Method 2: Using Pre-built Binaries** (Pending support)

### 2. What operating systems does Candy support?

Candy supports the following operating systems:
- Linux
- macOS
- Windows
- BSD systems

### 3. How do I run Candy?

```bash
# Use default configuration file (config.toml)
candy

# Use custom configuration file
candy -c /path/to/config.toml

# View help
candy -h
```

### 4. How do I configure virtual hosts?

Add multiple `[[host]]` blocks in `config.toml`:

```toml
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"

[[host.route]]
location = "/"
root = "./html"

[[host]]
ip = "0.0.0.0"
port = 80
server_name = "test.com"

[[host.route]]
location = "/"
root = "./test"
```

### 5. How do I configure HTTPS?

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"  # Certificate path
certificate_key = "./ssl/server.key"  # Private key path

[[host.route]]
location = "/"
root = "./html"
```

### 6. How do I generate a self-signed certificate?

```bash
openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout server.key -out server.crt
```

### 7. How do I configure reverse proxy?

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
```

### 8. How do I enable directory listing?

```toml
[[host.route]]
location = "/"
root = "./html"
auto_index = true  # Enable directory listing
```

## Troubleshooting

### 1. Cannot find configuration file

**Issue**: `Error: Failed to read config.toml: No such file or directory (os error 2)`

**Solution**:
- Ensure the configuration file is named `config.toml` and located in the current directory
- Or use the `-c` option to specify the configuration file path: `candy -c /path/to/config.toml`

### 2. Port is already in use

**Issue**: `Error: Address already in use (os error 48)`

**Solution**:
- Change the port number in the configuration file
- Or terminate the process that is using the port

### 3. Configuration file format error

**Issue**: `Error: TOML parse error at line 10, column 5`

**Solution**:
- Check if the configuration file syntax complies with TOML specifications
- Ensure all strings are properly quoted
- Check for unclosed parentheses or braces

### 4. Static files cannot be accessed

**Issue**: Page displays 404 error

**Solution**:
- Check if the `root` path is correct
- Ensure file permissions are correct (Candy needs read permission)
- Check if the `auto_index` option in the configuration file is enabled

### 5. SSL certificate validation failed

**Issue**: Browser shows certificate error

**Solution**:
- Ensure the certificate path is correct
- Check if the certificate has expired
- Use a valid CA-signed certificate (for production environments)

### 6. Reverse proxy timeout

**Issue**: `504 Gateway Timeout` error

**Solution**:
- Increase the `proxy_timeout` configuration value
- Check if the backend server responds normally
- Optimize backend server performance

### 7. Request body too large

**Issue**: `413 Request Entity Too Large` error

**Solution**:
- Increase the `max_body_size` configuration value
- Optimize client request size
- Consider using chunked upload

### 8. Lua script execution failure

**Issue**: Lua script does not work properly

**Solution**:
- Ensure compilation with `--features lua`
- Check Lua script syntax
- Check logs for detailed error information
- Ensure the script path is correct

### 9. Insufficient permissions

**Issue**: Cannot access certain files or ports

**Solution**:
- Ensure the running user has sufficient permissions
- For Linux/macOS, use `sudo` to elevate privileges
- For Windows, run as administrator

### 10. Log file cannot be written

**Issue**: Cannot create log file

**Solution**:
- Check if the `log_folder` path exists
- Ensure the running user has write permissions
- Check if disk space is sufficient

## Performance Optimization

### 1. Adjust worker process count

Candy uses the Tokio asynchronous runtime and defaults to using the number of system CPU cores.

### 2. Enable Gzip compression

Candy enables Gzip compression by default, but it can be adjusted:

```toml
# Configure at route level
[[host.route]]
location = "/"
root = "./html"
# Compression is enabled by default
```

### 3. Configure cache headers

```toml
[[host.route]]
location = "/static"
root = "./static"

[host.route.headers]
Cache-Control = "public, max-age=3600"
```

### 4. Use high-performance file system

Ensure static files are stored on high-performance storage (such as SSD).

### 5. Load balancing

Use upstream server groups and load balancing:

```toml
[[upstream]]
name = "backend"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },
    { server = "192.168.1.101:8080", weight = 1 }
]
```

## Security Best Practices

### 1. Use HTTPS

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"
```

### 2. Restrict access

Use Lua scripts for access control:

```toml
[[host.route]]
location = "/admin"
lua_script = "./scripts/auth.lua"
```

### 3. Set security headers

```toml
[host.headers]
X-Frame-Options = "DENY"
X-Content-Type-Options = "nosniff"
X-XSS-Protection = "1; mode=block"
```

### 4. Principle of least privilege

Run Candy as a non-root user:

```bash
# Create dedicated user
useradd -r -s /bin/false candy
chown -R candy:candy /path/to/candy
su -s /bin/bash -c "candy" candy
```

## Logging and Debugging

### 1. Configure log level

```toml
log_level = "debug"  # Options: trace, debug, info, warn, error
log_folder = "./logs"
```

### 2. View real-time logs

```bash
tail -f logs/candy.log
```

### 3. Enable detailed logs

```toml
log_level = "trace"
```

### 4. Check system resource usage

```bash
# Linux/macOS
top -p $(pgrep candy)

# Windows
tasklist /fi "imagename eq candy.exe"
```

## Deployment Recommendations

### 1. Use system service

**Linux (Systemd):**

```ini
[Unit]
Description=Candy Web Server
After=network.target

[Service]
Type=simple
User=candy
Group=candy
ExecStart=/usr/local/bin/candy -c /etc/candy/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

**macOS (Launchd):**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.candy.server</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/candy</string>
        <string>-c</string>
        <string>/etc/candy/config.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>/var/log/candy.err</string>
    <key>StandardOutPath</key>
    <string>/var/log/candy.out</string>
</dict>
</plist>
```

### 2. Use Docker

```dockerfile
FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/candy /usr/local/bin/
COPY config.toml /etc/candy/
EXPOSE 80 443
CMD ["candy", "-c", "/etc/candy/config.toml"]
```

## Contact and Support

If you encounter issues or need help:

1. Check [GitHub Issues](https://github.com/DefectingCat/candy/issues)
2. Submit a new issue
3. Check example configurations and documentation
4. Check CHANGELOG.md for latest updates

## Contributing

Contributions to the Candy project are welcome:

1. Fork the repository
2. Create a feature branch
3. Submit changes
4. Send a Pull Request

## License

Candy follows the MIT License. See the LICENSE file for details.
