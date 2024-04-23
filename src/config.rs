use crate::error::Result;
use config::Config;
use serde_derive::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    pub port: u32,
}

pub fn init_config() -> Result<Settings> {
    let config = Config::builder()
        .add_source(config::File::with_name("./config.toml"))
        .add_source(config::Environment::with_prefix("CANDY"))
        .build()?;

    let settings: Settings = config.try_deserialize()?;
    Ok(settings)
}
