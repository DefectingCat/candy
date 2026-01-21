// #![feature(iterator_try_collect)]

use anyhow::{Context, Result};

use clap::Parser;
use config::Settings;
use consts::{COMMIT, COMPILER};
use http::make_server;
use tracing::{debug, error, info};

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
    let handles = std::sync::Arc::new(std::sync::Mutex::new(handles));
    let handles_clone = handles.clone(); // 克隆一个副本用于闭包
    let config_path = args.config.clone();
    let stop_tx = start_config_watcher(&args.config, move |result| {
        match result {
            Ok(new_settings) => {
                info!("Config file reloaded successfully: {:?}", new_settings);
                info!("Config file changed, restarting servers to apply new config...");

                // 停止当前所有服务器
                if let Ok(mut current_handles) = handles_clone.lock() {
                    for handle in current_handles.iter() {
                        handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
                    }
                    current_handles.clear();
                    info!("All existing servers have been signaled to shut down");

                    // 在新的 tokio 任务中启动新服务器
                    let new_hosts = new_settings.host;
                    let handles_clone2 = handles_clone.clone();
                    let _config_path_clone = config_path.clone();
                    tokio::spawn(async move {
                        let mut new_handles = Vec::new();
                        for host in new_hosts {
                            match make_server(host).await {
                                Ok(handle) => {
                                    new_handles.push(handle);
                                    info!("New server instance started");
                                }
                                Err(e) => {
                                    error!("Failed to start new server instance: {:?}", e);
                                }
                            }
                        }

                        if let Ok(mut current_handles) = handles_clone2.lock() {
                            *current_handles = new_handles;
                            info!("All servers have been restarted successfully");
                        } else {
                            error!("Failed to acquire lock for server handles");
                        }
                    });
                } else {
                    error!("Failed to acquire lock for server handles");
                }
            }
            Err(e) => {
                error!("Failed to reload config file: {:?}", e);
            }
        }
    })?;

    info!("Server started");

    // 保持主线程运行，直到所有服务器停止
    tokio::signal::ctrl_c().await?;
    info!("Received Ctrl+C, shutting down");

    // 优雅关闭所有服务器
    if let Ok(mut current_handles) = handles.lock() {
        for handle in current_handles.iter() {
            handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
        }
        info!("All servers have been signaled to shut down");
        current_handles.clear();
        let _ = stop_tx
            .send(())
            .map_err(|err| error!("Send stop_tx failed: {:?}", err));
    } else {
        error!("Failed to acquire lock for server handles");
    }

    Ok(())
}
