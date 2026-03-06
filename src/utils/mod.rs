//! 工具模块
//!
//! 提供 Candy 服务器运行所需的工具函数和辅助功能。
//!
//! # 子模块
//!
//! - `config_watcher`：配置文件监听和自动重载
//! - `logging`：日志系统初始化和管理
//! - `service`：服务工具函数（如端口解析）
//!
//! # 主要功能
//!
//! - 日志系统初始化
//! - 配置文件监听
//! - 服务器优雅关闭
//! - 端口解析工具
//!
//! # 使用示例
//!
//! ```no_run
//! use candy::utils::{initialize_logger, start_config_watcher, shutdown_application};
//! use candy::config::Settings;
//!
//! // 初始化日志系统
//! let settings = Settings::new("config.toml").unwrap();
//! initialize_logger(&settings).await.unwrap();
//!
//! // 启动配置监听器
//! let stop_tx = start_config_watcher("config.toml", |result| {
//!     // 处理配置变更
//! }).unwrap();
//!
//! // 优雅关闭
//! shutdown_application(handles, stop_tx).await;
//! ```

//! 工具模块
//!
//! 提供 Candy 服务器运行所需的工具函数和辅助功能。
//!
//! # 子模块
//!
//! - `config_watcher`：配置文件监听和自动重载
//! - `logging`：日志系统初始化和管理
//! - `service`：服务工具函数（如端口解析）
//!
//! # 主要功能
//!
//! - 日志系统初始化
//! - 配置文件监听
//! - 服务器优雅关闭
//! - 端口解析工具
//!
//! # 使用示例
//!
//! ```no_run
//! use candy::utils::{initialize_logger, shutdown_application};
//! use candy::config::Settings;
//!
//! #[tokio::main]
//! async fn main() {
//!     // 初始化日志系统
//!     let settings = Settings::new("config.toml").unwrap();
//!     initialize_logger(&settings).await.unwrap();
//!     
//!     // 优雅关闭服务器
//!     // shutdown_application(handles, stop_tx).await;
//! }
//! ```

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
///
/// 根据配置初始化日志系统，支持文件日志和控制台输出。
///
/// # 参数
///
/// * `settings` - 服务器配置实例，包含日志级别和日志目录配置
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误信息。
///
/// # 日志配置
///
/// - 日志级别：trace/debug/info/warn/error
/// - 日志目录：默认为 `./logs`，按日期滚动
/// - 输出目标：文件 + 控制台
///
/// # 启动信息
///
/// 日志初始化成功后，会输出以下信息：
/// - 服务器名称和版本
/// - 编译器信息
/// - 操作系统和架构
/// - 配置信息（debug 模式）
///
/// # 示例
///
/// ```no_run
/// use candy::utils::initialize_logger;
/// use candy::config::Settings;
///
/// #[tokio::main]
/// async fn main() {
///     let settings = Settings::new("config.toml").unwrap();
///     initialize_logger(&settings).await.unwrap();
/// }
/// ```
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
///
/// 执行完整的服务器关闭流程，确保所有请求处理完成后再退出。
///
/// # 参数
///
/// * `handles` - 服务器句柄列表，用于发送关闭信号
/// * `stop_tx` - 配置监听器的停止信号发送器
///
/// # 关闭流程
///
/// 1. 记录关闭信号日志
/// 2. 对所有服务器发送优雅关闭信号
/// 3. 等待所有服务器完成正在处理的请求（最多 30 秒）
/// 4. 停止配置文件监听器
/// 5. 记录关闭完成日志
///
/// # 示例
///
/// ```no_run
/// use candy::utils::shutdown_application;
/// use std::sync::Arc;
/// use tokio::sync::{Mutex, oneshot};
///
/// #[tokio::main]
/// async fn main() {
///     // 假设已有 handles 和 stop_tx
///     // let handles = Arc::new(Mutex::new(vec![...]));
///     // let (stop_tx, _) = oneshot::channel();
///     
///     // 收到关闭信号
///     tokio::signal::ctrl_c().await.unwrap();
///     
///     // 优雅关闭
///     // shutdown_application(handles, stop_tx).await;
/// }
/// ```
///
/// # 注意
///
/// - 服务器会在 30 秒内完成正在处理的请求
/// - 如果 30 秒内未完成，服务器会强制关闭
/// - 配置监听器会立即停止
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
