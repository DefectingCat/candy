use crate::{consts::keep_alive_timeoutd_efault, error::Result};
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
    pub index: Vec<String>,
    /// HTTP keep-alive timeout
    #[serde(default = "keep_alive_timeoutd_efault")]
    pub keep_alive: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
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
