use crate::{
    consts::{
        default_compression_enabled, default_compression_level, default_disabled,
        default_log_folder, default_log_level, host_index, timeout_default,
        upstream_timeout_default,
    },
    error::Result,
};
use std::fs;
use std::path::Path;

use anyhow::Context;
use dashmap::DashMap;
use serde::Deserialize;

/// 上游服务器配置
#[derive(Deserialize, Clone, Debug)]
pub struct UpstreamServer {
    /// 服务器地址（IP:端口 或 域名:端口）
    pub server: String,
    /// 服务器权重（用于加权轮询，默认值为1）
    #[serde(default = "default_weight")]
    pub weight: u32,
}

/// 默认服务器权重
fn default_weight() -> u32 {
    1
}

/// 默认 Lua 代码缓存设置
fn default_lua_code_cache() -> bool {
    true
}

/// 负载均衡算法类型
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LoadBalanceType {
    /// 轮询算法（默认）
    RoundRobin,
    /// 加权轮询算法
    WeightedRoundRobin,
    /// IP哈希算法（会话保持）
    IpHash,
    /// 最少连接数算法
    LeastConn,
}

/// 默认负载均衡算法
fn default_load_balance() -> LoadBalanceType {
    LoadBalanceType::WeightedRoundRobin
}

/// 主动健康检查配置
#[derive(Deserialize, Clone, Debug, Default)]
#[allow(dead_code)]
pub struct HealthCheck {
    /// 健康检查间隔（毫秒），默认3000ms
    #[serde(default = "default_health_check_interval")]
    pub interval: u64,
    /// 连续失败次数阈值，达到该次数标记为down，默认3次
    #[serde(default = "default_health_check_fall")]
    pub fall: u32,
    /// 连续成功次数阈值，达到该次数标记为up，默认2次
    #[serde(default = "default_health_check_rise")]
    pub rise: u32,
    /// 健康检查协议类型，默认http
    #[serde(default = "default_health_check_type")]
    pub r#type: String,
    /// 发送的HTTP探测包内容，默认"HEAD / HTTP/1.0\r\n\r\n"
    #[serde(default = "default_health_check_send")]
    pub check_http_send: String,
    /// 期望返回的HTTP状态码，默认"200-399"
    #[serde(default = "default_health_check_expect")]
    pub check_http_expect_alive: String,
}

/// 默认健康检查间隔（毫秒）
fn default_health_check_interval() -> u64 {
    3000
}

/// 默认健康检查连续失败次数
fn default_health_check_fall() -> u32 {
    3
}

/// 默认健康检查连续成功次数
fn default_health_check_rise() -> u32 {
    2
}

/// 默认健康检查类型
fn default_health_check_type() -> String {
    "http".to_string()
}

/// 默认健康检查发送内容
fn default_health_check_send() -> String {
    "HEAD / HTTP/1.0\r\n\r\n".to_string()
}

/// 默认健康检查期望状态码范围
fn default_health_check_expect() -> String {
    "200-399".to_string()
}

/// 上游服务器组配置
#[derive(Deserialize, Clone, Debug)]
pub struct Upstream {
    /// 上游服务器组名称
    pub name: String,
    /// 服务器列表
    pub server: Vec<UpstreamServer>,
    /// 负载均衡算法类型
    #[serde(default = "default_load_balance")]
    pub method: LoadBalanceType,
    /// 被动健康检查：在fail_timeout时间内允许的最大失败次数，默认1次，0表示不检查
    #[serde(default = "default_max_fails")]
    pub max_fails: u32,
    /// 被动健康检查：失败超时时间（秒），也是服务器不可用的持续时间，默认10秒
    #[serde(default = "default_fail_timeout")]
    pub fail_timeout: u64,
    /// 主动健康检查配置
    pub health_check: Option<HealthCheck>,
}

/// 默认最大失败次数
fn default_max_fails() -> u32 {
    1
}

/// 默认失败超时时间（秒）
fn default_fail_timeout() -> u64 {
    10
}

/// 错误页面路由配置
#[derive(Deserialize, Clone, Debug)]
pub struct ErrorRoute {
    /// HTTP 状态码
    pub status: u16,
    /// 错误页面路径
    pub page: String,
}

/// 压缩配置
#[derive(Deserialize, Clone, Debug)]
pub struct CompressionConfig {
    /// 是否启用 gzip 压缩
    #[serde(default = "default_compression_enabled")]
    pub gzip: bool,
    /// 是否启用 deflate 压缩
    #[serde(default = "default_compression_enabled")]
    pub deflate: bool,
    /// 是否启用 brotli 压缩
    #[serde(default = "default_compression_enabled")]
    pub br: bool,
    /// 是否启用 zstd 压缩
    #[serde(default = "default_compression_enabled")]
    pub zstd: bool,
    /// 压缩级别 (1-9)，级别越高压缩率越高但速度越慢
    /// 1 = 最快压缩，9 = 最佳压缩，默认 6
    #[serde(default = "default_compression_level")]
    pub level: u8,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            gzip: true,
            deflate: true,
            br: true,
            zstd: true,
            level: 6,
        }
    }
}

/// 虚拟主机中的路由
/// 可以是静态文件、反向代理或正向代理
#[derive(Deserialize, Clone, Debug)]
pub struct SettingRoute {
    /// 路由位置
    /// 用于 axum 路由注册
    pub location: String,
    /// 静态资源根文件夹
    pub root: Option<String>,
    /// 是否启用目录列表
    #[serde(default = "default_disabled")]
    pub auto_index: bool,

    /// 索引文件格式列表
    #[serde(default = "host_index")]
    pub index: Vec<String>,
    /// 自定义错误页面
    pub error_page: Option<ErrorRoute>,
    /// 自定义 404 页面
    pub not_found_page: Option<ErrorRoute>,

    /// 反向代理 URL
    pub proxy_pass: Option<String>,
    /// 上游服务器组名称（用于负载均衡）
    pub upstream: Option<String>,
    /// 正向代理（设置为 true 启用）
    pub forward_proxy: Option<bool>,
    /// 连接上游服务器超时时间（秒）
    #[serde(default = "upstream_timeout_default")]
    pub proxy_timeout: u16,
    /// 请求最大 body 大小（字节）
    pub max_body_size: Option<u64>,

    /// HTTP 头部
    /// 用于覆盖配置中的头部
    pub headers: Option<HeaderMap>,

    /// Lua 脚本
    pub lua_script: Option<String>,
    /// 是否启用 Lua 代码缓存
    /// 默认为 true，启用缓存可以提高性能
    /// 设置为 false 时，每次请求都会重新编译 Lua 脚本
    #[serde(default = "default_lua_code_cache")]
    pub lua_code_cache: bool,

    /// HTTP 重定向目标 URL
    pub redirect_to: Option<String>,
    /// HTTP 重定向状态码
    pub redirect_code: Option<u16>,

    // --- 路由级别压缩配置 ---
    /// 是否启用 gzip 压缩（路由级别，覆盖全局配置）
    pub gzip: Option<bool>,
    /// 是否启用 deflate 压缩（路由级别，覆盖全局配置）
    pub deflate: Option<bool>,
    /// 是否启用 brotli 压缩（路由级别，覆盖全局配置）
    pub br: Option<bool>,
    /// 是否启用 zstd 压缩（路由级别，覆盖全局配置）
    pub zstd: Option<bool>,
    /// 压缩级别 1-9（路由级别，覆盖全局配置）
    pub level: Option<u8>,
}

/// 主机路由映射
/// 每个主机可以有多个路由
pub type HostRouteMap = DashMap<String, SettingRoute>;
/// HTTP 头部映射
pub type HeaderMap = DashMap<String, String>;

/// 虚拟主机配置
/// 每个主机可以监听一个端口和一个 IP 地址
/// 支持基于域名的路由配置
#[derive(Deserialize, Clone, Debug, Default)]
pub struct SettingHost {
    /// 主机 IP 地址
    pub ip: String,
    /// 主机端口
    pub port: u16,
    /// 服务器名称（域名）
    /// 用于支持同一端口下的不同域名路由
    /// 例如："rua.plus" 或 "www.rua.plus"
    pub server_name: Option<String>,
    /// 是否启用 SSL
    #[serde(default = "default_disabled")]
    pub ssl: bool,
    /// SSL 证书文件路径
    pub certificate: Option<String>,
    /// SSL 密钥文件路径
    pub certificate_key: Option<String>,
    /// 配置文件中的路由列表
    pub route: Vec<SettingRoute>,
    /// 主机路由从 Vec<SettingRoute> 转换为 DashMap<String, SettingRoute>
    /// {
    ///     "/doc": <SettingRoute>
    /// }
    #[serde(skip)]
    pub route_map: HostRouteMap,
    /// HTTP 保持连接超时时间（秒）
    #[serde(default = "timeout_default")]
    pub timeout: u16,
}

/// 完整的服务器配置
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Settings {
    /// 日志级别 (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// 日志文件夹路径
    #[serde(default = "default_log_folder")]
    pub log_folder: String,

    /// 压缩配置
    #[serde(default)]
    pub compression: CompressionConfig,

    /// 上游服务器组配置
    pub upstream: Option<Vec<Upstream>>,

    /// 虚拟主机列表
    pub host: Vec<SettingHost>,
}

impl Settings {
    /// 根据名称查找上游服务器组配置
    #[allow(dead_code)]
    pub fn find_upstream(&self, name: &str) -> Option<&Upstream> {
        self.upstream.as_ref()?.iter().find(|u| u.name == name)
    }

    /// 从 TOML 配置文件创建 Settings 实例
    ///
    /// # 参数
    ///
    /// * `path` - 配置文件路径
    ///
    /// # 返回值
    ///
    /// 解析后的 Settings 实例，或包含错误信息的 Result
    pub fn new(path: &str) -> Result<Self> {
        let file = fs::read_to_string(path).with_context(|| format!("Failed to read {path}"))?;
        let mut settings: Settings = toml::from_str(&file)?;

        // 初始化路由映射
        for host in &mut settings.host {
            host.route_map = HostRouteMap::new();
            for route in &host.route {
                host.route_map.insert(route.location.clone(), route.clone());
            }
        }

        // 验证配置
        settings.validate()?;

        Ok(settings)
    }

    /// 验证配置的有效性
    fn validate(&self) -> Result<()> {
        // 验证上游服务器组配置
        if let Some(upstreams) = &self.upstream {
            for (i, upstream) in upstreams.iter().enumerate() {
                if upstream.name.is_empty() {
                    return Err(anyhow::anyhow!("Upstream {} has empty name", i).into());
                }

                if upstream.server.is_empty() {
                    return Err(anyhow::anyhow!("Upstream {} has no servers", i).into());
                }

                for (j, server) in upstream.server.iter().enumerate() {
                    if server.server.is_empty() {
                        return Err(anyhow::anyhow!(
                            "Upstream {} server {} has empty address",
                            i,
                            j
                        )
                        .into());
                    }

                    // 验证服务器地址格式 (应该包含主机和端口)
                    if !server.server.contains(':') {
                        return Err(anyhow::anyhow!(
                            "Upstream {} server {} address invalid: missing port (format should be host:port)",
                            i,
                            j
                        )
                        .into());
                    }
                }
            }
        }

        for (i, host) in self.host.iter().enumerate() {
            // 验证 SSL 配置
            if host.ssl {
                if host.certificate.is_none() || host.certificate_key.is_none() {
                    return Err(anyhow::anyhow!(
                        "Host {} has SSL enabled but missing certificate or key",
                        i
                    )
                    .into());
                }

                // 验证证书文件存在
                if let Some(cert_path) = &host.certificate
                    && !Path::new(cert_path).exists()
                {
                    return Err(anyhow::anyhow!(
                        "Host {} certificate file not found: {}",
                        i,
                        cert_path
                    )
                    .into());
                }

                if let Some(key_path) = &host.certificate_key
                    && !Path::new(key_path).exists()
                {
                    return Err(anyhow::anyhow!(
                        "Host {} certificate key file not found: {}",
                        i,
                        key_path
                    )
                    .into());
                }
            }

            // 验证路由配置
            for (j, route) in host.route.iter().enumerate() {
                if route.location.is_empty() || !route.location.starts_with('/') {
                    return Err(anyhow::anyhow!(
                        "Host {} route {} location invalid: {}",
                        i,
                        j,
                        route.location
                    )
                    .into());
                }

                // 验证至少有一个有效的路由配置
                let has_valid_route = route.root.is_some()
                    || route.proxy_pass.is_some()
                    || route.upstream.is_some()
                    || route.forward_proxy.is_some() && route.forward_proxy.unwrap()
                    || cfg!(feature = "lua") && route.lua_script.is_some()
                    || route.redirect_to.is_some();

                if !has_valid_route {
                    return Err(anyhow::anyhow!("Host {} route {} configuration invalid (requires root, proxy_pass, upstream, lua_script or redirect_to)", i, j).into());
                }

                // 如果配置了 upstream，验证该 upstream 存在
                if let Some(upstream_name) = &route.upstream
                    && self
                        .upstream
                        .as_ref()
                        .and_then(|u| u.iter().find(|x| x.name == *upstream_name))
                        .is_none()
                {
                    return Err(anyhow::anyhow!(
                        "Host {} route {} references unknown upstream: {}",
                        i,
                        j,
                        upstream_name
                    )
                    .into());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_settings_new() {
        // Create a temporary TOML config file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            default_type = "text/plain"
            types = {{ "txt" = "text/plain", "html" = "text/html" }}

            [[host]]
            ip = "127.0.0.1"
            port = 8080
            ssl = false
            timeout = 30

            [[host.route]]
            location = "/"
            root = "/var/www"
            index = ["index.html", "index.txt"]
            proxy_timeout = 10
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        // Verify host settings
        let host = &settings.host[0];
        assert_eq!(host.ip, "127.0.0.1");
        assert_eq!(host.port, 8080);
        assert_eq!(host.timeout, 30);

        // Verify route settings
        let route = &host.route[0];
        assert_eq!(route.location, "/");
        assert_eq!(route.root, Some("/var/www".to_string()));
        assert_eq!(route.proxy_timeout, 10);
    }

    #[test]
    fn test_settings_missing_file() {
        let result = Settings::new("nonexistent.toml");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read nonexistent.toml")
        );
    }

    #[test]
    fn test_settings_invalid_toml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "invalid toml content").unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_settings_ssl_missing_cert() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 443
            ssl = true

            [[host.route]]
            location = "/"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("missing certificate or key")
        );
    }

    #[test]
    fn test_settings_ssl_cert_not_found() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 443
            ssl = true
            certificate = "nonexistent.crt"
            certificate_key = "nonexistent.key"

            [[host.route]]
            location = "/"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("certificate file not found")
        );
    }

    #[test]
    fn test_settings_invalid_route_location() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 8080
            ssl = false

            [[host.route]]
            location = "invalid"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("location invalid"));
    }

    #[test]
    fn test_settings_invalid_route_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 8080
            ssl = false

            [[host.route]]
            location = "/"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("configuration invalid")
        );
    }

    #[test]
    fn test_settings_complete_config() {
        // Create temporary certificate files for test
        let mut cert_file = NamedTempFile::new().unwrap();
        writeln!(cert_file, "dummy certificate").unwrap();
        let cert_path = cert_file.path().to_str().unwrap().to_string();

        let mut key_file = NamedTempFile::new().unwrap();
        writeln!(key_file, "dummy key").unwrap();
        let key_path = key_file.path().to_str().unwrap().to_string();

        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            log_level = "debug"
            log_folder = "/var/log/candy"

            [[host]]
            ip = "127.0.0.1"
            port = 8080
            ssl = false
            timeout = 60

            [[host.route]]
            location = "/"
            root = "/var/www"
            index = ["index.html", "index.htm"]
            auto_index = true

            [[host.route]]
            location = "/api"
            proxy_pass = "http://localhost:3000"
            proxy_timeout = 30
            max_body_size = 1048576

            [[host]]
            ip = "0.0.0.0"
            port = 443
            ssl = true
            certificate = "{}"
            certificate_key = "{}"
            timeout = 30

            [[host.route]]
            location = "/"
            root = "/var/www/ssl"
            error_page = {{ status = 404, page = "/404.html" }}
            "#,
            cert_path, key_path
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        // Verify global settings
        assert_eq!(settings.log_level, "debug");
        assert_eq!(settings.log_folder, "/var/log/candy");

        // Verify first host
        assert_eq!(settings.host.len(), 2);
        let host1 = &settings.host[0];
        assert_eq!(host1.ip, "127.0.0.1");
        assert_eq!(host1.port, 8080);
        assert!(!host1.ssl);
        assert_eq!(host1.timeout, 60);
        assert_eq!(host1.route.len(), 2);
        assert!(host1.route_map.contains_key("/"));
        assert!(host1.route_map.contains_key("/api"));

        // Verify second host
        let host2 = &settings.host[1];
        assert_eq!(host2.ip, "0.0.0.0");
        assert_eq!(host2.port, 443);
        assert!(host2.ssl);
        assert_eq!(host2.certificate, Some(cert_path));
        assert_eq!(host2.certificate_key, Some(key_path));
    }

    #[test]
    fn test_settings_default_values() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        // Verify default values
        assert!(!settings.host[0].ssl);
        assert!(!settings.host[0].route[0].auto_index); // default_disabled is false
    }

    #[test]
    fn test_upstream_config() {
        // 创建包含 upstream 配置的临时 TOML 文件
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[upstream]]
            name = "test_backend"
            server = [
                {{ server = "192.168.1.100:8080" }},
                {{ server = "192.168.1.101:8080" }}
            ]

            [[host]]
            ip = "127.0.0.1"
            port = 8080
            ssl = false

            [[host.route]]
            location = "/api"
            upstream = "test_backend"
            proxy_timeout = 30
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        // 验证 upstream 配置
        assert!(settings.upstream.is_some());
        let upstreams = settings.upstream.as_ref().unwrap();
        assert_eq!(upstreams.len(), 1);

        let test_backend = &upstreams[0];
        assert_eq!(test_backend.name, "test_backend");
        assert_eq!(test_backend.server.len(), 2);
        assert_eq!(test_backend.server[0].server, "192.168.1.100:8080");
        assert_eq!(test_backend.server[1].server, "192.168.1.101:8080");

        // 验证路由配置
        assert_eq!(settings.host.len(), 1);
        let route = &settings.host[0].route[0];
        assert_eq!(route.location, "/api");
        assert_eq!(route.upstream, Some("test_backend".to_string()));
        assert_eq!(route.proxy_timeout, 30);
    }

    #[test]
    fn test_invalid_upstream_config() {
        // 测试无效的 upstream 配置（空服务器列表）
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[upstream]]
            name = "invalid_backend"
            server = []

            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            upstream = "invalid_backend"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_upstream_config() {
        // 测试引用不存在的 upstream
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            upstream = "nonexistent_backend"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
    }

    // ========== CompressionConfig Tests ==========

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();

        assert!(config.gzip);
        assert!(config.deflate);
        assert!(config.br);
        assert!(config.zstd);
        assert_eq!(config.level, 6);
    }

    #[test]
    fn test_compression_config_default_values() {
        // 测试 serde 默认值函数
        assert!(default_compression_enabled());
        assert_eq!(default_compression_level(), 6);
    }

    #[test]
    fn test_compression_config_deserialize_default() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        // 验证压缩配置使用默认值
        let compression = &settings.compression;
        assert!(compression.gzip);
        assert!(compression.deflate);
        assert!(compression.br);
        assert!(compression.zstd);
        assert_eq!(compression.level, 6);
    }

    #[test]
    fn test_compression_config_deserialize_custom() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [compression]
            gzip = false
            deflate = true
            br = false
            zstd = true
            level = 9

            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        let compression = &settings.compression;
        assert!(!compression.gzip);
        assert!(compression.deflate);
        assert!(!compression.br);
        assert!(compression.zstd);
        assert_eq!(compression.level, 9);
    }

    #[test]
    fn test_compression_config_partial_deserialize() {
        // 部分字段使用默认值
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [compression]
            gzip = false
            level = 3

            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        let compression = &settings.compression;
        assert!(!compression.gzip);
        assert!(compression.deflate); // 使用默认值
        assert!(compression.br); // 使用默认值
        assert!(compression.zstd); // 使用默认值
        assert_eq!(compression.level, 3);
    }

    #[test]
    fn test_compression_config_level_boundaries() {
        // 测试压缩级别边界值
        for level in [1u8, 5, 9] {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(
                file,
                r#"
                [compression]
                level = {}

                [[host]]
                ip = "127.0.0.1"
                port = 8080

                [[host.route]]
                location = "/"
                root = "/var/www"
                "#,
                level
            )
            .unwrap();

            let path = file.path().to_str().unwrap();
            let settings = Settings::new(path).unwrap();
            assert_eq!(settings.compression.level, level);
        }
    }

    #[test]
    fn test_compression_config_all_disabled() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [compression]
            gzip = false
            deflate = false
            br = false
            zstd = false
            level = 1

            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            root = "/var/www"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        let compression = &settings.compression;
        assert!(!compression.gzip);
        assert!(!compression.deflate);
        assert!(!compression.br);
        assert!(!compression.zstd);
        assert_eq!(compression.level, 1);
    }

    #[test]
    fn test_route_level_compression_config() {
        // 测试路由级别的压缩配置
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [compression]
            gzip = true
            level = 6

            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/api"
            root = "/var/www"
            gzip = false
            level = 9
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        // 验证全局压缩配置
        assert!(settings.compression.gzip);
        assert_eq!(settings.compression.level, 6);

        // 验证路由级别的压缩配置
        let route = &settings.host[0].route[0];
        assert_eq!(route.location, "/api");
        assert_eq!(route.gzip, Some(false));
        assert_eq!(route.level, Some(9));
    }

    #[test]
    fn test_compression_config_clone_debug() {
        let config = CompressionConfig {
            gzip: true,
            deflate: false,
            br: true,
            zstd: false,
            level: 8,
        };

        let cloned = config.clone();
        assert_eq!(config.gzip, cloned.gzip);
        assert_eq!(config.deflate, cloned.deflate);
        assert_eq!(config.br, cloned.br);
        assert_eq!(config.zstd, cloned.zstd);
        assert_eq!(config.level, cloned.level);

        // 测试 Debug trait
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("CompressionConfig"));
        assert!(debug_str.contains("gzip"));
    }
}
