use crate::{
    consts::{host_index, insert_default_mimes, mime_default, timeout_default, types_default},
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

#[derive(Deserialize, Clone, Debug)]
pub struct SettingRoute {
    /// The register route
    pub location: String,
    /// The static assets root folder
    pub root: Option<String>,
    /// Index files format
    #[serde(default = "host_index")]
    pub index: Vec<String>,
    pub error_page: Option<ErrorRoute>,
    // reverse proxy url
    pub proxy_pass: Option<String>,
}

pub type HostRouteMap = BTreeMap<String, SettingRoute>;

#[derive(Deserialize, Clone, Debug)]
pub struct SettingHost {
    pub ip: String,
    pub port: u32,
    route: Vec<Option<SettingRoute>>,
    #[serde(skip_deserializing, skip_serializing)]
    pub route_map: HostRouteMap,
    /// HTTP keep-alive timeout
    #[serde(default = "timeout_default")]
    pub timeout: u16,
    pub headers: Option<BTreeMap<String, String>>,
}

pub type MIMEType = BTreeMap<Cow<'static, str>, Cow<'static, str>>;

#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    #[serde(default = "mime_default")]
    pub default_type: Cow<'static, str>,
    #[serde(default = "types_default")]
    pub types: MIMEType,
    pub host: Vec<SettingHost>,
}

pub fn init_config(path: &str) -> Result<Settings> {
    let file = fs::read_to_string(path).with_context(|| format!("read {path} failed"))?;
    let mut settings: Settings = toml::from_str(&file)?;

    // convert route map
    settings.host.iter_mut().for_each(|host| {
        let routes = &mut host.route;
        for route in routes.iter_mut() {
            if route.is_none() {
                continue;
            }
            let route = route.take().unwrap();
            host.route_map.insert(route.location.to_string(), route);
        }
    });

    // combine mime types
    insert_default_mimes(&mut settings.types);

    Ok(settings)
}
