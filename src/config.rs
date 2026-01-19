use crate::{
    consts::{
        default_disabled, default_log_folder, default_log_level, host_index, timeout_default,
        upstream_timeout_default,
    },
    error::Result,
};
use std::fs;
use std::path::Path;

use anyhow::Context;
use dashmap::DashMap;
use serde::Deserialize;

/// 错误页面路由配置
#[derive(Deserialize, Clone, Debug)]
pub struct ErrorRoute {
    /// HTTP 状态码
    pub status: u16,
    /// 错误页面路径
    pub page: String,
}

/// 虚拟主机中的路由
/// 可以是静态文件或反向代理
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

    /// HTTP 重定向目标 URL
    pub redirect_to: Option<String>,
    /// HTTP 重定向状态码
    pub redirect_code: Option<u16>,
}

/// 主机路由映射
/// 每个主机可以有多个路由
pub type HostRouteMap = DashMap<String, SettingRoute>;
/// HTTP 头部映射
pub type HeaderMap = DashMap<String, String>;

/// 虚拟主机配置
/// 每个主机可以监听一个端口和一个 IP 地址
#[derive(Deserialize, Clone, Debug, Default)]
pub struct SettingHost {
    /// 主机 IP 地址
    pub ip: String,
    /// 主机端口
    pub port: u16,
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

    /// 虚拟主机列表
    pub host: Vec<SettingHost>,
}

impl Settings {
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
        let file = fs::read_to_string(path).with_context(|| format!("读取 {path} 失败"))?;
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
        for (i, host) in self.host.iter().enumerate() {
            // 验证 SSL 配置
            if host.ssl {
                if host.certificate.is_none() || host.certificate_key.is_none() {
                    return Err(anyhow::anyhow!("主机 {} 启用了 SSL 但缺少证书或密钥", i).into());
                }

                // 验证证书文件存在
                if let Some(cert_path) = &host.certificate
                    && !Path::new(cert_path).exists()
                {
                    return Err(anyhow::anyhow!("主机 {} 证书文件未找到: {}", i, cert_path).into());
                }

                if let Some(key_path) = &host.certificate_key
                    && !Path::new(key_path).exists()
                {
                    return Err(
                        anyhow::anyhow!("主机 {} 证书密钥文件未找到: {}", i, key_path).into(),
                    );
                }
            }

            // 验证路由配置
            for (j, route) in host.route.iter().enumerate() {
                if route.location.is_empty() || !route.location.starts_with('/') {
                    return Err(anyhow::anyhow!(
                        "主机 {} 路由 {} 位置无效: {}",
                        i,
                        j,
                        route.location
                    )
                    .into());
                }

                // 验证至少有一个有效的路由配置
                let has_valid_route = route.root.is_some()
                    || route.proxy_pass.is_some()
                    || route.lua_script.is_some()
                    || route.redirect_to.is_some();

                if !has_valid_route {
                    return Err(anyhow::anyhow!("主机 {} 路由 {} 配置无效（需要 root、proxy_pass、lua_script 或 redirect_to）", i, j).into());
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
                .contains("读取 nonexistent.toml 失败")
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
        assert!(result.unwrap_err().to_string().contains("缺少证书或密钥"));
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
        assert!(result.unwrap_err().to_string().contains("证书文件未找到"));
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
        assert!(result.unwrap_err().to_string().contains("位置无效"));
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
        assert!(result.unwrap_err().to_string().contains("配置无效"));
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
}
