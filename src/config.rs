use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// 主配置结构体
#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub tls: Option<TlsConfig>,
    pub http: HttpConfig,
    pub log: LogConfig,
}

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// HTTP 监听地址（可选，用于重定向）
    pub http_listen: Option<SocketAddr>,
    /// HTTPS 监听地址
    pub https_listen: SocketAddr,
    /// Worker 进程数
    pub workers: usize,
    /// 静态文件根目录
    pub root: PathBuf,
}

/// TLS 配置
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// 是否启用 TLS
    pub enabled: bool,
    /// 证书文件路径
    pub cert: PathBuf,
    /// 私钥文件路径
    pub key: PathBuf,
}

/// HTTP 配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HttpConfig {
    /// 最大请求头大小
    pub max_header_size: usize,
    /// 最大请求体大小
    pub max_body_size: usize,
    /// Keep-Alive 超时（秒）
    pub keep_alive_timeout: u64,
    /// 是否启用 HTTP/2 Server Push
    pub http2_push: bool,
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 是否记录访问日志
    pub access: bool,
    /// 日志格式: "combined" 或 "json"
    pub format: LogFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Combined,
    Json,
}

/// 配置错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Invalid listen address: {0}")]
    InvalidAddress(String),

    #[error("Invalid root path: {0}")]
    InvalidRootPath(String),

    #[error("TLS enabled but certificate not found: {0}")]
    TlsCertNotFound(PathBuf),

    #[error("TLS enabled but key not found: {0}")]
    TlsKeyNotFound(PathBuf),

    #[error("Invalid worker count: {0}")]
    InvalidWorkerCount(usize),
}

impl Config {
    /// 从 TOML 文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::parse(&content)
    }

    /// 从 TOML 字符串解析配置
    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(content)?;

        // 验证并转换
        let server = Self::parse_server_config(&raw.server)?;
        let tls = Self::parse_tls_config(raw.tls.as_ref())?;
        let http = Self::parse_http_config(raw.http.as_ref());
        let log = Self::parse_log_config(raw.log.as_ref());

        Ok(Config {
            server,
            tls,
            http,
            log,
        })
    }

    fn parse_server_config(raw: &RawServerConfig) -> Result<ServerConfig, ConfigError> {
        // 解析 HTTPS 地址（主监听地址）
        let https_listen: SocketAddr = raw
            .listen
            .parse()
            .map_err(|_| ConfigError::InvalidAddress(raw.listen.clone()))?;

        // 解析 HTTP 地址（可选，用于重定向）
        let http_listen = raw
            .http_listen
            .as_ref()
            .map(|s| s.parse::<SocketAddr>())
            .transpose()
            .map_err(|_| ConfigError::InvalidAddress(raw.http_listen.clone().unwrap()))?;

        let root = PathBuf::from(&raw.root);
        if !root.exists() {
            return Err(ConfigError::InvalidRootPath(raw.root.clone()));
        }

        let workers = raw.workers.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
        });

        if workers == 0 {
            return Err(ConfigError::InvalidWorkerCount(workers));
        }

        Ok(ServerConfig {
            http_listen,
            https_listen,
            workers,
            root,
        })
    }

    fn parse_tls_config(raw: Option<&RawTlsConfig>) -> Result<Option<TlsConfig>, ConfigError> {
        let Some(raw) = raw else {
            return Ok(None);
        };

        if !raw.enabled.unwrap_or(false) {
            return Ok(None);
        }

        let cert = PathBuf::from(&raw.cert);
        let key = PathBuf::from(&raw.key);

        if !cert.exists() {
            return Err(ConfigError::TlsCertNotFound(cert));
        }
        if !key.exists() {
            return Err(ConfigError::TlsKeyNotFound(key));
        }

        Ok(Some(TlsConfig {
            enabled: true,
            cert,
            key,
        }))
    }

    fn parse_http_config(raw: Option<&RawHttpConfig>) -> HttpConfig {
        match raw {
            Some(r) => HttpConfig {
                max_header_size: r.max_header_size.unwrap_or(8192),
                max_body_size: r.max_body_size.unwrap_or(10 * 1024 * 1024), // 默认 10MB
                keep_alive_timeout: r.keep_alive_timeout.unwrap_or(60),
                http2_push: r.http2_push.unwrap_or(false), // 默认禁用
            },
            None => HttpConfig {
                max_header_size: 8192,
                max_body_size: 10 * 1024 * 1024, // 默认 10MB
                keep_alive_timeout: 60,
                http2_push: false,
            },
        }
    }

    fn parse_log_config(raw: Option<&RawLogConfig>) -> LogConfig {
        match raw {
            Some(r) => {
                let format = match r.format.as_deref() {
                    Some("json") => LogFormat::Json,
                    _ => LogFormat::Combined,
                };
                LogConfig {
                    access: r.access.unwrap_or(true),
                    format,
                }
            }
            None => LogConfig {
                access: true,
                format: LogFormat::Combined,
            },
        }
    }
}

// ========== RAW 配置结构体（用于 TOML 反序列化）==========

#[derive(Debug, serde::Deserialize)]
struct RawConfig {
    server: RawServerConfig,
    tls: Option<RawTlsConfig>,
    http: Option<RawHttpConfig>,
    log: Option<RawLogConfig>,
}

#[derive(Debug, serde::Deserialize)]
struct RawServerConfig {
    listen: String,
    http_listen: Option<String>,
    workers: Option<usize>,
    root: String,
}

#[derive(Debug, serde::Deserialize)]
struct RawTlsConfig {
    enabled: Option<bool>,
    cert: String,
    key: String,
}

#[derive(Debug, serde::Deserialize)]
struct RawHttpConfig {
    max_header_size: Option<usize>,
    max_body_size: Option<usize>,
    keep_alive_timeout: Option<u64>,
    http2_push: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct RawLogConfig {
    access: Option<bool>,
    format: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path().to_str().unwrap();

        let toml = format!(
            r#"
[server]
listen = "127.0.0.1:8080"
root = "{}"

[log]
access = true
"#,
            root
        );

        let config = Config::parse(&toml).unwrap();
        assert_eq!(config.server.https_listen.port(), 8080);
        assert!(config.server.http_listen.is_none());
        assert!(config.tls.is_none());
        assert!(config.log.access);
    }

    #[test]
    fn test_parse_invalid_address() {
        let toml = r#"
[server]
listen = "invalid-address"
root = "/tmp"
"#;
        let result = Config::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tls_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cert_path = temp_dir.path().join("cert.pem");
        let key_path = temp_dir.path().join("key.pem");
        std::fs::write(&cert_path, "").unwrap();
        std::fs::write(&key_path, "").unwrap();

        let toml = format!(
            r#"
[server]
listen = "127.0.0.1:443"
root = "/tmp"

[tls]
enabled = true
cert = "{}"
key = "{}"
"#,
            cert_path.display(),
            key_path.display()
        );

        let config = Config::parse(&toml).unwrap();
        assert!(config.tls.is_some());
        let tls = config.tls.unwrap();
        assert!(tls.enabled);
    }
}
