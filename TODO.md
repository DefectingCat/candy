# TODO

## Configuration

- [x] Overwrite headers in config
- [x] Config init tests
- [x] Error page
- [x] Custom error page with stats code
- [x] Logging to file
  - [ ] logrotate
- [ ] Benches
  - [ ] Docker with oha
- [x] Max body size
- [x] HTTP 2

## Features

- [x] DNS Support
- [x] Feature flags to disable some functions
  - [x] Lua support
- [x] Config file hot reload
- [ ] Congigura compression method in config file
  - [ ] zstd
  - [ ] gzip
  - [ ] deflate
  - [ ] br
- [x] Cross platform compile
  - [x] x86_64-unknown-linux-gnu
  - [x] x86_64-unknown-linux-musl
  - [x] aarch64-unknown-linux-gnu
  - [x] aarch64-unknown-linux-musl
  - [x] x86_64-pc-windows-gnu
  - [x] x86_64-unknown-freebsd
  - [x] loongarch64-unknown-linux-gnu
- [x] HTTP redirect
- [x] Load balance
  - [x] Round Robin (Default)
  - [x] Weighted Round Robin
  - [x] IP Hash (ip_hash)
  - [ ] Least Connections (least_conn)
  - [ ] Backend Server Health Check
- [x] Proxy
- [x] Reverse Proxy
  - [x] Connect to upstream timeout setting
  - [x] Follow http 301
  - [x] Custom headers
- [x] SSL
- [x] Cli
- [x] Specific custom config location
- [x] HTTP 2
- [x] Lua engine
- [ ] JavaScript engine
- [ ] HTTP 3
- [ ] Specify thread numbers
- [x] Dockerization
- [x] Docs
- [x] Multiple virtual hosts
- [x] Build with compile info
- [x] Refactor HTTP 1
- [x] Graceful shutdown
- [x] `keep-alive` timeout setting
- [x] HTTP Etag: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag#etag_value>
- [x] 304 Not Modified
- [x] List directory
- [x] Content compress
  - [x] zstd
  - [x] gzip
  - [x] deflate
  - [x] br
