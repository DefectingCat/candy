use crate::error::Result;
use std::{fs, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct SettingRoute {
    pub location: String,
    pub root: PathBuf,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SettingHost {
    pub ip: String,
    pub port: u32,
    pub route: SettingRoute,
    pub index: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    pub host: Vec<SettingHost>,
}

pub fn init_config() -> Result<Settings> {
    let file = fs::read_to_string("./config.toml")?;
    let settings: Settings = toml::from_str(&file)?;
    Ok(settings)
}
