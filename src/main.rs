// #![feature(iterator_try_collect)]

use anyhow::{Context, Result};

use clap::Parser;
use config::Settings;
use consts::{COMMIT, COMPILER};
use http::make_server;
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
    let mut handles = Vec::new();
    for host in hosts {
        let handle = make_server(host).await?;
        handles.push(handle);
    }

    // 启动配置文件监听
    let _config_path = args.config.clone();
    let handles = std::sync::Arc::new(handles);
    let _stop_tx = start_config_watcher(&args.config, move || {
        info!("Config file changed, stopping servers...");
        // 停止所有服务器
        for handle in handles.iter() {
            handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
        }
        info!("All servers have been signaled to shut down");
    })?;

    info!("server started");

    // 保持主线程运行，直到所有服务器停止
    tokio::signal::ctrl_c().await?;
    info!("Received Ctrl+C, shutting down");

    Ok(())
}
