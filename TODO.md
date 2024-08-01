## TODO

-   [ ] Dockerization
-   [ ] Docs
-   [x] Build with compile info
-   [x] Refactor HTTP 1
-   [x] Graceful shutdown
-   [x] `keep-alive` timeout setting
-   [x] HTTP Etag: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag#etag_value
-   [x] Content compress
    -   [x] zstd
    -   [x] gzip
    -   [x] deflate
    -   [x] br

### Configuration

-   [x] File MIME type
-   [x] Overwrite headers in config
-   [x] Config init tests
-   [x] Error page
-   [ ] Logging to file
-   [ ] Benchs
-   [ ] Max body size

### Features

-   [x] Cross platform compile
    -   [x] x86_64-unknown-linux-gnu
    -   [x] x86_64-unknown-linux-musl
    -   [x] aarch64-unknown-linux-gnu
    -   [x] aarch64-unknown-linux-musl
    -   [x] x86_64-pc-windows-gnu
    -   [x] x86_64-unknown-freebsd
    -   [x] loongarch64-unknown-linux-gnu
-   [ ] Proxy
-   [x] Reverse Proxy
    -   [ ] Connect to upstream timeout setting
-   [ ] FastCGI
-   [ ] SSL
-   [x] Cli
-   [x] Specific custom config location
-   [ ] HTTP 2
-   [ ] HTTP 3
-   [ ] Specify thread numbers
