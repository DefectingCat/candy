use std::sync::Arc;

use anyhow::{Context, Result};
use std::net::SocketAddr;
use tokio::sync::{Mutex, oneshot};
use tracing::{debug, error, info};

use crate::config::Settings;
use crate::consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};
use crate::http::shutdown_servers;
use axum_server::Handle;

pub mod config_watcher;
pub mod logging;
pub mod service;

pub use config_watcher::*;
pub use logging::*;
pub use service::*;

/// 初始化日志系统
pub async fn initialize_logger(settings: &Settings) -> Result<()> {
    let _guard = init_logger(settings.log_level.as_str(), settings.log_folder.as_str())
        .with_context(|| "Failed to initialize logger")?;
    info!("{} v{} ({})", NAME, VERSION, COMMIT);
    info!("Compiler: {}", COMPILER);
    info!("OS: {} {}", OS, ARCH);
    debug!("Configuration: {:?}", settings);
    Ok(())
}

/// 优雅关闭服务器和配置监听器
pub async fn shutdown_application(
    handles: Arc<Mutex<Vec<Handle<SocketAddr>>>>,
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
