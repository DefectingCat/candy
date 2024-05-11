use std::{process::exit, sync::OnceLock};

use anyhow::{Context, Result};

use tokio::task::JoinSet;
use tracing::{debug, error, info};

use crate::{
    config::{init_config, Settings},
    consts::{ARCH, NAME, OS, VERSION},
    utils::init_logger,
};

mod config;
mod consts;
mod error;
mod http;
mod service;
mod utils;

static SETTINGS: OnceLock<Settings> = OnceLock::new();
pub fn get_settings() -> &'static Settings {
    SETTINGS.get_or_init(|| {
        init_config()
            .with_context(|| "init config failed")
            .map_err(|err| {
                error!("{err}");
                exit(1);
            })
            .unwrap()
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();

    // global config
    let settings = get_settings();
    debug!("settings {:?}", settings);
    info!("{}/{}", NAME, VERSION);
    info!("OS: {} {}", OS, ARCH);

    let mut servers = settings
        .host
        .iter()
        .map(|host| host.mk_server())
        .collect::<JoinSet<_>>();

    info!("server started");

    while let Some(res) = servers.join_next().await {
        res??;
    }

    Ok(())
}
