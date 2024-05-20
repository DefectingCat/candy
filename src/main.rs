use anyhow::{anyhow, Context, Result};

use clap::Parser;
use tokio::task::JoinSet;
use tracing::{debug, info};

use crate::{
    config::init_config,
    consts::{get_settings, ARCH, NAME, OS, SETTINGS, VERSION},
    utils::init_logger,
};

mod cli;
mod config;
mod consts;
mod error;
mod http;
mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();
    init_logger();
    let settings = init_config(&args.config).with_context(|| "init config failed")?;
    SETTINGS
        .set(settings)
        .map_err(|err| anyhow!("init config failed {err:?}"))?;

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
