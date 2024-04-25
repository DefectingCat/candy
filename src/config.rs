use crate::error::Result;
use std::{collections::BTreeMap, fs};

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct SettingRoute {
    pub location: String,
    pub root: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SettingHost {
    pub ip: String,
    pub port: u32,
    route: Vec<Option<SettingRoute>>,
    #[serde(skip_deserializing, skip_serializing)]
    pub route_map: BTreeMap<String, SettingRoute>,
    pub index: Vec<String>,
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
