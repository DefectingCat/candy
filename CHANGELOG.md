# Changelog

## 0.2.3 - 2025-08-14

Features:

- Logging to file
- Add custom headers in route
- Add HTTP redirect support

Fix:

- Fix auto_index file path render error
- Fix auto_index rewrite error
- Fix cannot write logs into file

## 0.2.2 - 2025-07-03

Features:

- Support lua script
- Add max body size limit

## 0.2.1 - 2025-06-24

Features:

- `auto-index` support
- Stable rust version

## 0.2.0 - 2025-06-17

Features:

- Reverse proxy
- Refactor with axum
- SSL support

## 0.1.1 - 2024-07-02

Features:

- GitLab CI integration
- FreeBSD support
- Reverse proxy
- Connection timeout

Break changes:

- Remove `keep-alive` setting
- Add `timeout` setting

Fix:

- Internal server errror handler
- Custom error page
- Config tests

## 0.1.0 - 2024-05-13

Features:

- Graceful shutdown
- `keep-alive` timeout setting
- HTTP Etag: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag#etag_value>
- Content compress
  - zstd
  - gzip
  - deflate
  - br
- Stream file
- Stream content compress
