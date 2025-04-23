## TODO

- [ ] Dockerization
- [ ] Docs
- [x] Build with compile info
- [x] Refactor HTTP 1
- [x] Graceful shutdown
- [x] `keep-alive` timeout setting
- [ ] HTTP Etag: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag#etag_value
- [x] Content compress
    - [x] zstd
    - [x] gzip
    - [x] deflate
    - [x] br

### Configuration

- [ ] File MIME type
- [x] Overwrite headers in config
- [x] Config init tests
- [ ] Error page
- [ ] Custom error page with stats code
- [ ] Logging to file
- [ ] Benchs
- [ ] Max body size
- [x] HTTP 2

### Features

- [x] Cross platform compile
    - [x] x86_64-unknown-linux-gnu
    - [x] x86_64-unknown-linux-musl
    - [x] aarch64-unknown-linux-gnu
    - [x] aarch64-unknown-linux-musl
    - [x] x86_64-pc-windows-gnu
    - [x] x86_64-unknown-freebsd
    - [x] loongarch64-unknown-linux-gnu
- [ ] Load balance
- [ ] Proxy
- [ ] Reverse Proxy
    - [x] Connect to upstream timeout setting
    - [x] Follow http 301
    - [x] Custom headers
- [x] SSL
- [x] Cli
- [x] Specific custom config location
- [x] HTTP 2
- [ ] HTTP 3
- [ ] Specify thread numbers
