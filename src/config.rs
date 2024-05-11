use crate::{
    consts::{host_index, keep_alive_timeoutd_efault, mime_default, process_timeout},
    error::Result,
};
use std::{collections::BTreeMap, fs};

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct SettingRoute {
    /// The register route
    pub location: String,
    /// The static assets root folder
    pub root: String,
}

pub type HostRouteMap = BTreeMap<String, SettingRoute>;

#[derive(Deserialize, Clone, Debug)]
pub struct SettingHost {
    pub ip: String,
    pub port: u32,
    route: Vec<Option<SettingRoute>>,
    #[serde(skip_deserializing, skip_serializing)]
    pub route_map: BTreeMap<String, SettingRoute>,
    /// Index files format
    #[serde(default = "host_index")]
    pub index: Vec<String>,
    /// HTTP keep-alive timeout
    #[serde(default = "keep_alive_timeoutd_efault")]
    pub keep_alive: u16,
    // http process max timeout
    #[serde(default = "process_timeout")]
    pub process_timeout: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    #[serde(default = "mime_default")]
    pub default_type: String,
    pub host: Vec<SettingHost>,
}

pub fn init_config() -> Result<Settings> {
    let file = fs::read_to_string("./config.toml")?;
    let mut settings: Settings = toml::from_str(&file)?;

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

    Ok(settings)
}
