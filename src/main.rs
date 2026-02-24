use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use config::Settings;
use consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};
use http::{shutdown_servers, start_servers};
use tokio::sync::{Mutex, oneshot};
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

/// 初始化日志系统
async fn initialize_logger(settings: &Settings) -> Result<()> {
    let _guard = init_logger(settings.log_level.as_str(), settings.log_folder.as_str())
        .with_context(|| "Failed to initialize logger")?;
    info!("{} v{} ({})", NAME, VERSION, COMMIT);
    info!("Compiler: {}", COMPILER);
    info!("OS: {} {}", OS, ARCH);
    debug!("Configuration: {:?}", settings);
    Ok(())
}

/// 加载上游服务器配置到全局存储
fn load_upstreams(settings: &Settings) {
    crate::http::UPSTREAMS.clear();
    if let Some(upstreams) = &settings.upstream {
        for upstream in upstreams {
            crate::http::UPSTREAMS.insert(upstream.name.clone(), upstream.clone());
        }
    }
}

/// 启动初始服务器实例
async fn start_initial_servers(
    settings: Settings,
) -> Result<Arc<Mutex<Vec<axum_server::Handle<SocketAddr>>>>> {
    let handles = start_servers(settings.host).await;
    Ok(Arc::new(Mutex::new(handles)))
}

/// 处理配置文件变更的回调函数
async fn handle_config_change(
    result: crate::error::Result<Settings>,
    handles: Arc<Mutex<Vec<axum_server::Handle<SocketAddr>>>>,
) {
    match result {
        Ok(new_settings) => {
            info!("Config file reloaded successfully");
            info!("Config file changed, restarting servers to apply new config...");

            // 停止当前所有服务器
            let mut current_handles = handles.lock().await;
            shutdown_servers(&mut current_handles).await;

            // 在新的 tokio 任务中启动新服务器
            let new_hosts = new_settings.host;
            let new_upstreams = new_settings.upstream;
            let handles_clone = handles.clone();
            tokio::spawn(async move {
                // 清空全局配置存储，确保新配置完全生效
                crate::http::HOSTS.clear();
                crate::http::UPSTREAMS.clear();

                // 重新加载上游服务器配置
                if let Some(upstreams) = &new_upstreams {
                    for upstream in upstreams {
                        crate::http::UPSTREAMS.insert(upstream.name.clone(), upstream.clone());
                    }
                }

                let new_handles = start_servers(new_hosts).await;

                let mut current_handles = handles_clone.lock().await;
                *current_handles = new_handles;
                info!("All servers have been restarted successfully");
            });
        }
        Err(e) => {
            error!("Failed to reload config file: {:?}", e);
        }
    }
}

/// 优雅关闭服务器和配置监听器
async fn shutdown_application(
    handles: Arc<Mutex<Vec<axum_server::Handle<SocketAddr>>>>,
    stop_tx: oneshot::Sender<()>,
) {
    info!("Received shutdown signal, closing servers...");

    // 优雅关闭所有服务器
    let mut current_handles = handles.lock().await;
    shutdown_servers(&mut current_handles).await;

    // 停止配置监听
    if let Err(err) = stop_tx.send(()) {
        error!("Failed to send stop signal to config watcher: {:?}", err);
    }

    info!("Application shutdown complete");
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    // 加载和验证配置
    let settings =
        Settings::new(&args.config).with_context(|| "Failed to initialize configuration")?;

    // 初始化日志系统
    initialize_logger(&settings).await?;

    // 加载上游服务器配置
    load_upstreams(&settings);

    // 启动初始服务器
    let handles = start_initial_servers(settings).await?;

    // 启动配置文件监听
    let handles_clone = handles.clone();
    let stop_tx = start_config_watcher(&args.config, move |result| {
        let handles_clone = handles_clone.clone();
        Box::pin(handle_config_change(result, handles_clone))
    })?;

    info!("Server started");

    // 保持主线程运行，直到收到停止信号
    tokio::signal::ctrl_c().await?;

    // 优雅关闭应用程序
    shutdown_application(handles, stop_tx).await;

    Ok(())
}
