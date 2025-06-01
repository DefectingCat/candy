#![feature(iterator_try_collect)]

use anyhow::{Context, Result};
use tracing::error;

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();
    init_logger();

    let settings = Settings::new(&args.config).with_context(|| "init config failed")?;
    debug!("settings {:?}", settings);
    info!("{}/{} {}", NAME, VERSION, COMMIT);
    info!("{}", COMPILER);
    info!("OS: {} {}", OS, ARCH);

    let hosts = settings.host;
    let mut servers = hosts.into_iter().map(make_server).collect::<JoinSet<_>>();

    info!("server started");

    while let Some(res) = servers.join_next().await {
        match res {
            Ok(err) => {
                err.map_err(|err| error!("server error: {}", err)).ok();
            }
            Err(err) => {
                error!("server error: {}", err);
            }
        }
    }

    Ok(())
}
