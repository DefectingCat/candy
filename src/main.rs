use anyhow::{Context, Result};

use clap::Parser;
use config::Settings;
use consts::{COMMIT, COMPILER};
use http::make_server;
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
mod middlewares;
mod utils;

// static SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| RwLock::new(Settings::default()));

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();
    init_logger();

    // {
    //     let mut settings = SETTINGS.write().await;
    //     *settings = Settings::new(&args.config).with_context(|| "init config failed")?;
    // }
    // let settings = SETTINGS.read().await;

    let settings = Settings::new(&args.config).with_context(|| "init config failed")?;
    debug!("settings {:?}", settings);
    info!("{}/{} {}", NAME, VERSION, COMMIT);
    info!("{}", COMPILER);
    info!("OS: {} {}", OS, ARCH);

    let hosts = settings.host;
    let mut servers = hosts.into_iter().map(make_server).collect::<JoinSet<_>>();

    // settings.host = vec![];

    info!("server started");

    while let Some(res) = servers.join_next().await {
        res??;
    }

    Ok(())
}
