use anyhow::{Context, Result};

use clap::Parser;
use config::Settings;
use consts::{COMMIT, COMPILER};
use tokio::task::JoinSet;
use tracing::{debug, info};

use crate::{
    consts::{ARCH, NAME, OS, VERSION},
    utils::init_logger,
};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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
    let settings = Settings::new(&args.config).with_context(|| "init config failed")?;

    debug!("settings {:?}", settings);
    info!("{}/{} {}", NAME, VERSION, COMMIT);
    info!("{}", COMPILER);
    info!("OS: {} {}", OS, ARCH);

    let mut servers = settings
        .host
        .into_iter()
        .map(|host| host.mk_server())
        .collect::<JoinSet<_>>();

    info!("server started");

    while let Some(res) = servers.join_next().await {
        res??;
    }

    Ok(())
}
