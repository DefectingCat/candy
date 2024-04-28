use std::{collections::BTreeMap, sync::OnceLock, sync::RwLock};

use anyhow::{Context, Result};

use tokio::task::JoinSet;
use tracing::{debug, info};

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

static CACHE: OnceLock<RwLock<BTreeMap<String, u64>>> = OnceLock::new();
pub fn get_cache() -> &'static RwLock<BTreeMap<String, u64>> {
    CACHE.get_or_init(|| RwLock::new(BTreeMap::new()))
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();

    // global config
    let settings = init_config().with_context(|| "init config failed")?;
    let settings: &'static Settings = Box::leak(Box::new(settings));
    debug!("settings {:?}", settings);
    info!("{}/{}", NAME, VERSION);
    info!("OS: {} {}", OS, ARCH);

    // global cache

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
