# Changelog

## 0.2.0

Features:

- Reverse proxy
- Refactor with axum

## 0.1.1

Features:

- Gitlab CI integration
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

## 0.1.0

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
