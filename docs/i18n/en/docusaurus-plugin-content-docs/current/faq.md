---
sidebar_label: 常见问题
sidebar_position: 5
title: 常见问题与故障排除
---

## 常见问题

### 1. 如何安装 Candy？

**方法一：从源代码编译**

```bash
git clone https://github.com/DefectingCat/candy.git
cd candy
cargo build --release
```

**方法二：使用预编译二进制文件**（待支持）

### 2. Candy 支持哪些操作系统？

Candy 支持以下操作系统：
- Linux
- macOS
- Windows
- BSD 系统

### 3. 如何运行 Candy？

```bash
# 使用默认配置文件 (config.toml)
candy

# 使用自定义配置文件
candy -c /path/to/config.toml

# 查看帮助
candy -h
```

### 4. 如何配置虚拟主机？

在 `config.toml` 中添加多个 `[[host]]` 块：

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

### 5. 如何配置 HTTPS？

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"  # 证书路径
certificate_key = "./ssl/server.key"  # 私钥路径

[[host.route]]
location = "/"
root = "./html"
```

### 6. 如何生成自签名证书？

```bash
openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout server.key -out server.crt
```

### 7. 如何配置反向代理？

```toml
[[host.route]]
location = "/api"
proxy_pass = "http://localhost:3000"
```

### 8. 如何启用目录列表？

```toml
[[host.route]]
location = "/"
root = "./html"
auto_index = true  # 启用目录列表
```

## 故障排除

### 1. 无法找到配置文件

**问题**：`Error: Failed to read config.toml: No such file or directory (os error 2)`

**解决方案**：
- 确保配置文件名为 `config.toml` 并位于当前目录
- 或者使用 `-c` 选项指定配置文件路径：`candy -c /path/to/config.toml`

### 2. 端口被占用

**问题**：`Error: Address already in use (os error 48)`

**解决方案**：
- 更改配置文件中的端口号
- 或者终止占用该端口的进程

### 3. 配置文件格式错误

**问题**：`Error: TOML parse error at line 10, column 5`

**解决方案**：
- 检查配置文件语法是否符合 TOML 规范
- 确保所有字符串被正确引用
- 检查是否有未闭合的括号或大括号

### 4. 静态文件无法访问

**问题**：访问页面显示 404 错误

**解决方案**：
- 检查 `root` 路径是否正确
- 确保文件权限正确（Candy 需要读取权限）
- 检查配置文件中的 `auto_index` 选项是否启用

### 5. SSL 证书验证失败

**问题**：浏览器显示证书错误

**解决方案**：
- 确保证书路径正确
- 检查证书是否过期
- 使用有效的 CA 签名证书（生产环境）

### 6. 反向代理超时

**问题**：`504 Gateway Timeout` 错误

**解决方案**：
- 增加 `proxy_timeout` 配置值
- 检查后端服务器是否响应正常
- 优化后端服务器性能

### 7. 请求体过大

**问题**：`413 Request Entity Too Large` 错误

**解决方案**：
- 增加 `max_body_size` 配置值
- 优化客户端请求大小
- 考虑使用分块上传

### 8. Lua 脚本执行失败

**问题**：Lua 脚本无法正常工作

**解决方案**：
- 确保使用 `--features lua` 编译
- 检查 Lua 脚本语法
- 查看日志获取详细错误信息
- 确保脚本路径正确

### 9. 权限不足

**问题**：无法访问某些文件或端口

**解决方案**：
- 确保运行用户有足够权限
- 对于 Linux/macOS，使用 `sudo` 提升权限
- 对于 Windows，以管理员身份运行

### 10. 日志文件无法写入

**问题**：无法创建日志文件

**解决方案**：
- 检查 `log_folder` 路径是否存在
- 确保运行用户有写入权限
- 检查磁盘空间是否充足

## 性能优化

### 1. 调整工作进程数

Candy 使用 Tokio 异步运行时，默认使用系统 CPU 核心数。

### 2. 启用 Gzip 压缩

Candy 默认启用 Gzip 压缩，但可以调整：

```toml
# 在路由级别配置
[[host.route]]
location = "/"
root = "./html"
# 压缩已默认启用
```

### 3. 配置缓存头

```toml
[[host.route]]
location = "/static"
root = "./static"

[host.route.headers]
Cache-Control = "public, max-age=3600"
```

### 4. 使用高性能文件系统

确保静态文件存储在高性能存储上（如 SSD）。

### 5. 负载均衡

使用上游服务器组和负载均衡：

```toml
[[upstream]]
name = "backend"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },
    { server = "192.168.1.101:8080", weight = 1 }
]
```

## 安全最佳实践

### 1. 使用 HTTPS

```toml
[[host]]
ip = "0.0.0.0"
port = 443
ssl = true
certificate = "./ssl/server.crt"
certificate_key = "./ssl/server.key"
```

### 2. 限制访问

使用 Lua 脚本进行访问控制：

```toml
[[host.route]]
location = "/admin"
lua_script = "./scripts/auth.lua"
```

### 3. 设置安全头

```toml
[host.headers]
X-Frame-Options = "DENY"
X-Content-Type-Options = "nosniff"
X-XSS-Protection = "1; mode=block"
```

### 4. 最小权限原则

以非 root 用户运行 Candy：

```bash
# 创建专用用户
useradd -r -s /bin/false candy
chown -R candy:candy /path/to/candy
su -s /bin/bash -c "candy" candy
```

## 日志和调试

### 1. 配置日志级别

```toml
log_level = "debug"  # 可选：trace, debug, info, warn, error
log_folder = "./logs"
```

### 2. 查看实时日志

```bash
tail -f logs/candy.log
```

### 3. 启用详细日志

```toml
log_level = "trace"
```

### 4. 检查系统资源使用

```bash
# Linux/macOS
top -p $(pgrep candy)

# Windows
tasklist /fi "imagename eq candy.exe"
```

## 部署建议

### 1. 使用系统服务

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

### 2. 使用 Docker

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

## 联系和支持

如果您遇到问题或需要帮助：

1. 查看 [GitHub Issues](https://github.com/DefectingCat/candy/issues)
2. 提交新问题
3. 查看示例配置和文档
4. 检查 CHANGELOG.md 了解最新更新

## 贡献

欢迎为 Candy 项目做出贡献：

1. Fork 仓库
2. 创建功能分支
3. 提交更改
4. 发送 Pull Request

## 许可证

Candy 遵循 MIT 许可证。详情请查看 LICENSE 文件。
