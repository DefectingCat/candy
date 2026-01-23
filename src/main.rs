use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use config::Settings;
use consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};
use http::{make_server, shutdown_servers, start_servers};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::utils::{init_logger, start_config_watcher};

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

    let settings =
        Settings::new(&args.config).with_context(|| "Failed to initialize configuration")?;

    let _guard = init_logger(settings.log_level.as_str(), settings.log_folder.as_str())
        .with_context(|| "Failed to initialize logger")?;

    debug!("Configuration: {:?}", settings);
    info!("{} v{} ({})", NAME, VERSION, COMMIT);
    info!("Compiler: {}", COMPILER);
    info!("OS: {} {}", OS, ARCH);

    // 启动初始服务器
    let mut handles = Vec::new();
    for host in settings.host {
        let handle = make_server(host).await?;
        handles.push(handle);
    }

    // 启动配置文件监听
    let handles = Arc::new(Mutex::new(handles));
    let handles_clone = handles.clone();
    let stop_tx = start_config_watcher(&args.config, move |result| {
        let handles_clone = handles_clone.clone();
        Box::pin(async move {
            match result {
                Ok(new_settings) => {
                    info!("Config file reloaded successfully: {:?}", new_settings);
                    info!("Config file changed, restarting servers to apply new config...");

                    // 停止当前所有服务器
                    let mut current_handles = handles_clone.lock().await;
                    shutdown_servers(&mut current_handles).await;

                    // 在新的 tokio 任务中启动新服务器
                    let new_hosts = new_settings.host;
                    let handles_clone2 = handles_clone.clone();
                    tokio::spawn(async move {
                        // 清空全局 HOSTS 变量，确保新配置完全生效
                        crate::http::HOSTS.clear();

                        let new_handles = start_servers(new_hosts).await;

                        let mut current_handles = handles_clone2.lock().await;
                        *current_handles = new_handles;
                        info!("All servers have been restarted successfully");
                    });
                }
                Err(e) => {
                    error!("Failed to reload config file: {:?}", e);
                }
            }
        })
    })?;

    info!("Server started");

    // 保持主线程运行，直到收到停止信号
    tokio::signal::ctrl_c().await?;
    info!("Received Ctrl+C, shutting down");

    // 优雅关闭所有服务器
    let mut current_handles = handles.lock().await;
    shutdown_servers(&mut current_handles).await;

    // 停止配置监听
    if let Err(err) = stop_tx.send(()) {
        error!("Failed to send stop signal to config watcher: {:?}", err);
    }

    Ok(())
}
