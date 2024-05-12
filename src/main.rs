use anyhow::{Context, Result};

use tokio::task::JoinSet;
use tracing::{debug, info};

use crate::{
    config::init_config,
    consts::{get_settings, ARCH, NAME, OS, VERSION},
    utils::init_logger,
};

mod config;
mod consts;
mod error;
mod http;
mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    init_config().with_context(|| "init config failed")?;

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
