use anyhow::{Context, Result};

use tokio::task::JoinSet;
use tracing::{debug, info};

use crate::{
    config::{init_config, Settings},
    utils::init_logger,
};

mod config;
mod error;
mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    let settings = init_config().with_context(|| "init config failed")?;
    let settings: &'static Settings = Box::leak(Box::new(settings));
    debug!("settings {:?}", settings);

    let mut servers = settings
        .host
        .iter()
        .map(|host| host.mk_server())
        .collect::<JoinSet<_>>();

    info!("Server started");

    while let Some(res) = servers.join_next().await {
        res??;
    }

    Ok(())
}
