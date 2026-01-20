// #![feature(iterator_try_collect)]

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
    utils::{init_logger, start_config_watcher},
};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod cli;
mod config;
mod consts;
mod error;
mod http;
#[cfg(feature = "lua")]
mod lua_engine;
mod middlewares;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    let settings = Settings::new(&args.config).with_context(|| "init config failed")?;

    let _guard = init_logger(settings.log_level.as_str(), settings.log_folder.as_str())
        .with_context(|| "init logger failed")?;

    debug!("settings {:?}", settings);
    info!("{}/{} {}", NAME, VERSION, COMMIT);
    info!("{}", COMPILER);
    info!("OS: {} {}", OS, ARCH);

    let hosts = settings.host;
    let mut servers = hosts.into_iter().map(make_server).collect::<JoinSet<_>>();

    // 启动配置文件监听
    let config_path = args.config.clone();
    let _stop_tx = start_config_watcher(&args.config, move || {
        info!("Config file changed, reloading...");
        match Settings::new(&config_path) {
            Ok(new_settings) => {
                info!("Config reloaded successfully: {:?}", new_settings);
                // 这里可以添加配置重载后的逻辑，例如重启服务器等
            }
            Err(e) => {
                error!("Failed to reload config: {:?}", e);
            }
        }
    })?;

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
