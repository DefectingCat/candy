use crate::{
    consts::{
        host_index, insert_default_mimes, mime_default, timeout_default, types_default,
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
    pub location: String,
    /// The static assets root folder
    pub root: Option<String>,
    /// Index files format
    #[serde(default = "host_index")]
    pub index: Vec<String>,
    /// Custom error page
    pub error_page: Option<ErrorRoute>,

    /// Reverse proxy url
    pub proxy_pass: Option<String>,
    /// Timeout for connect to upstream
    #[serde(default = "upstream_timeout_default")]
    pub proxy_timeout: u16,
}

/// Host routes
/// Each host can have multiple routes
pub type HostRouteMap = BTreeMap<String, SettingRoute>;

/// Virtual host
/// Each host can listen on one port and one ip
#[derive(Deserialize, Clone, Debug)]
pub struct SettingHost {
    /// Host ip
    pub ip: String,
    /// Host port
    pub port: u32,
    /// SSL certificate location
    pub certificate: Option<String>,
    /// ssl key location
    pub certificate_key: Option<String>,
    route: Vec<Option<SettingRoute>>,
    /// Host route map
    #[serde(skip_deserializing, skip_serializing)]
    pub route_map: HostRouteMap,
    /// HTTP keep-alive timeout
    #[serde(default = "timeout_default")]
    pub timeout: u16,
    /// HTTP headers
    /// Used to overwrite headers in config
    pub headers: Option<BTreeMap<String, String>>,
}

pub type MIMEType = BTreeMap<Cow<'static, str>, Cow<'static, str>>;

/// Whole config settings
#[derive(Deserialize, Clone, Debug)]
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
        let mut settings: Settings = toml::from_str(&file)?;

        // convert route map
        settings.host.iter_mut().for_each(|host| {
            host.route
                .iter_mut()
                .filter_map(Option::take)
                .for_each(|route| {
                    host.route_map.insert(route.location.to_string(), route);
                });
        });

        // combine mime types
        insert_default_mimes(&mut settings.types);

        Ok(settings)
    }
}
