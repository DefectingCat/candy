use crate::{
    consts::{
        default_disabled, host_index, mime_default, timeout_default, types_default,
        upstream_timeout_default,
    },
    error::Result,
};
use std::{borrow::Cow, collections::BTreeMap, fs};

use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct ErrorRoute {
    pub status: u16,
    pub page: String,
}

/// Route in virtual host
/// Can be a static file or a reverse proxy
#[derive(Deserialize, Clone, Debug)]
pub struct SettingRoute {
    /// The register route
    /// for axum route
    pub location: String,
    /// The static assets root folder
    pub root: Option<String>,
    /// Index files format
    #[serde(default = "host_index")]
    pub index: Vec<String>,
    /// Custom error page
    pub error_page: Option<ErrorRoute>,
    /// Custom 404 page
    pub not_found_page: Option<ErrorRoute>,

    /// Reverse proxy url
    pub proxy_pass: Option<String>,
    /// Timeout for connect to upstream
    #[serde(default = "upstream_timeout_default")]
    pub proxy_timeout: u16,
}

/// Host routes
/// Each host can have multiple routes
pub type HostRouteMap = BTreeMap<String, SettingRoute>;
/// headers
pub type HeaderMap = BTreeMap<String, String>;

/// Virtual host
/// Each host can listen on one port and one ip
#[derive(Deserialize, Clone, Debug, Default)]
pub struct SettingHost {
    /// Host ip
    pub ip: String,
    /// Host port
    pub port: u16,
    /// SSL enable
    #[serde(default = "default_disabled")]
    pub ssl: bool,
    /// SSL certificate location
    pub certificate: Option<String>,
    /// ssl key location
    pub certificate_key: Option<String>,
    /// Routes in config file
    pub route: Vec<SettingRoute>,
    /// Host routes convert from Vec<SettingRoute> to BTreeMap<String, SettingRoute>
    /// {
    ///     "/doc": <SettingRoute>
    /// }
    #[serde(skip)]
    pub route_map: BTreeMap<String, SettingRoute>,
    /// HTTP keep-alive timeout
    #[serde(default = "timeout_default")]
    pub timeout: u16,
    /// HTTP headers
    /// Used to overwrite headers in config
    pub headers: Option<HeaderMap>,
}

pub type MIMEType = BTreeMap<Cow<'static, str>, Cow<'static, str>>;

/// Whole config settings
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Settings {
    /// Default file type for unknow file
    #[serde(default = "mime_default")]
    pub default_type: Cow<'static, str>,
    /// MIME types
    #[serde(default = "types_default")]
    pub types: MIMEType,
    /// Virtual host
    pub host: Vec<SettingHost>,
}

impl Settings {
    pub fn new(path: &str) -> Result<Self> {
        let file = fs::read_to_string(path).with_context(|| format!("read {path} failed"))?;
        let settings: Settings = toml::from_str(&file)?;
        Ok(settings)
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

        // Verify default values
        assert_eq!(settings.default_type, "text/plain");
        assert_eq!(settings.types.len(), 2);

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
                .contains("read nonexistent.toml failed")
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
}
