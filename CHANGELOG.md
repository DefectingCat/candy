# Changelog

## [0.2.5] - 2025-02-24

### Added
- **Library crate support**: Added library crate support and tests (6e6f8e0)
- **Load balancing algorithms**: Added support for round_robin, weighted_round_robin, and ip_hash load balancing methods (d98bff0, 5be42f6, 02c1398)
- **Forward proxy**: Added forward proxy support (ea69745)
- **Config validation**: Added configuration validation and proxy timeout handling (ea121c6)
- **JSON schema**: Added JSON schema for configuration file (4474991)
- **Unit tests**: Added comprehensive unit tests for reverse_proxy.rs, config_watcher.rs, and simple utility functions (1147480, 645ab03, b6e9b0a)
- **Debug logging**: Enhanced debug logging with file and line number information (42ae888)
- **Custom error handling**: Improved error handling for custom errors (ca0c9a5)
- **CLI documentation**: Updated CLI `long_about` with comprehensive feature list (6aecb81)

### Fixed
- **IP hash overflow**: Prevented overflow in ip_hash function (02c1398)
- **Config watcher robustness**: Improved config watcher reliability with retry mechanisms and better async handling (5749821, 727d777, 85bb857)
- **Server startup tests**: Fixed server startup test issues (6fdc389, 5c6a20d)
- **Shutdown handling**: Improved server shutdown handling and fixed config reload closure variable capture (1a45318)
- **Config reload**: Enhanced config reload and server restart handling with async callbacks and tokio mutex (6b9a385, ec81732)

### Changed
- **Dependencies**: Updated all dependencies to latest versions (39d68e3) and optimized by using specific features instead of 'full' (4511f07)
- **Code organization**: Refactored main.rs, moving auxiliary methods to application module for improved code organization (376aaab, 1e8e234, 2d150c1)
- **Config watcher**: Enhanced config file watcher with debouncing mechanism and configurable parameters (ef64ad8, b538ba3)
- **Docker configuration**: Improved Dockerfile and configuration for minimal image size (0e5a80a, 391d26c)
- **Documentation**: Comprehensive updates to documentation:
  - Updated CLAUDE.md with development rules and Clippy warnings (4e7a4c0)
  - Added Chinese documentation and completed English README (34d1488)
  - Updated CONTRIBUTING.md with Chinese guidelines (f37ba91)
  - Enhanced README with comprehensive feature list (34d1488)

### Performance
- **Memory optimization**: Optimized dependencies by using specific features instead of 'full' (4511f07)
- **Async handling**: Improved async handling in config_watcher using futures to simplify type definitions (727d777)

### Refactor
- **Route registration**: Refactored route registration logic for cleaner code structure (56ae93d)
- **Server startup**: Refactored server startup and shutdown logic (70cdbcc)
- **Static file serving**: Refactored src/http/serve.rs for cleaner code structure (1d74387)
- **Config watcher**: Removed useless clones and improved error handling (6086e2d)
- **Logging initialization**: Improved logger initialization with better error handling (d57c9a9, 5d4bdb1)

### Development
- **Clippy warnings**: Fixed all Clippy warnings (4e7a4c0)
- **Code formatting**: Ensured all code follows Rust formatting guidelines

## 0.2.4 - 2026-01-19

Features:
- Add Lua support as an optional feature with feature flag
- Support domain-based routing configuration
- Improve configuration module with validation and route mapping initialization
- Add Lua engine unit tests
- Update dependencies (axum-server, reqwest, and others)

Fix:
- Fix MDX compilation error in intro.md

Docs:
- Update documentation: improve formatting, add installation instructions and quick examples
- Update AGENTS.md with comprehensive guidelines
- Complete documentation for Lua script functionality
- Improve configuration file documentation
- Translate error messages and logs from Chinese to English

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
