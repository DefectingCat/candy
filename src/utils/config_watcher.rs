use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::oneshot;
use tokio::time::{self, Duration, Instant};
use tracing::{error, info};

use crate::config::Settings;
use crate::error::Result;

/// 配置变更回调函数类型
pub type ConfigChangeCallback = dyn Fn(Result<Settings>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
    + Send
    + Sync
    + 'static;

/// 配置监听器的参数
#[derive(Debug, Clone)]
pub struct ConfigWatcherConfig {
    /// 防抖时间（毫秒），避免文件频繁变更导致的重复处理
    pub debounce_ms: u64,
    /// 重命名/删除事件后等待文件稳定的时间（毫秒）
    pub rewatch_delay_ms: u64,
    /// 读取配置和重新 watch 的最大重试次数
    pub max_retries: usize,
    /// 重试之间的延迟（毫秒）
    pub retry_delay_ms: u64,
    /// 事件监听超时时间（秒）
    pub poll_timeout_secs: u64,
}

impl Default for ConfigWatcherConfig {
    fn default() -> Self {
        ConfigWatcherConfig {
            debounce_ms: 500,
            rewatch_delay_ms: 800,
            max_retries: 5,
            retry_delay_ms: 100,
            poll_timeout_secs: 1,
        }
    }
}

/// 启动配置文件监听（简化版本，保持向后兼容）
///
/// # 参数
///
/// * `config_path` - 配置文件路径
/// * `callback` - 配置文件变化时的回调函数，参数为重新读取后的配置
///
/// # 返回值
///
/// 返回一个发送器，用于发送停止信号
pub fn start_config_watcher(
    config_path: impl AsRef<Path>,
    callback: impl Fn(
        Result<Settings>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
    + Send
    + Sync
    + 'static,
) -> Result<oneshot::Sender<()>, notify::Error> {
    start_config_watcher_with_config(config_path, callback, None)
}

/// 启动配置文件监听（带配置参数版本）
///
/// # 参数
///
/// * `config_path` - 配置文件路径
/// * `callback` - 配置文件变化时的回调函数，参数为重新读取后的配置
/// * `config` - 监听器配置参数（可选，使用默认值）
///
/// # 返回值
///
/// 返回一个发送器，用于发送停止信号
pub fn start_config_watcher_with_config(
    config_path: impl AsRef<Path>,
    callback: impl Fn(
        Result<Settings>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
    + Send
    + Sync
    + 'static,
    watcher_config: Option<ConfigWatcherConfig>,
) -> Result<oneshot::Sender<()>, notify::Error> {
    let (stop_tx, stop_rx) = oneshot::channel();
    let config_path = config_path.as_ref().to_owned();
    let watcher_config = watcher_config.unwrap_or_default();
    let callback = std::sync::Arc::new(callback) as std::sync::Arc<ConfigChangeCallback>;

    tokio::spawn(async move {
        if let Err(e) = run_watcher(config_path, callback, watcher_config, stop_rx).await {
            error!("Config watcher failed: {:?}", e);
        }
    });

    Ok(stop_tx)
}

/// 内部执行监听器逻辑的函数
async fn run_watcher(
    config_path: std::path::PathBuf,
    callback: std::sync::Arc<ConfigChangeCallback>,
    config: ConfigWatcherConfig,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<(), notify::Error> {
    let (tx, rx) = std::sync::mpsc::channel();
    let rx = std::sync::Arc::new(std::sync::Mutex::new(rx)); // 使用 Arc+Mutex 包装 rx
    let watcher = std::sync::Arc::new(std::sync::Mutex::new(Box::new(notify::recommended_watcher(
        tx,
    )?) as Box<dyn Watcher + Send>)); // 包装 watcher 并确保它是 Send 的

    // 初始 watch
    watcher
        .lock()
        .unwrap()
        .watch(&config_path, RecursiveMode::NonRecursive)?;
    info!("Watching config file: {:?}", config_path);

    let mut last_event_time = Instant::now();
    let debounce_duration = Duration::from_millis(config.debounce_ms);
    let poll_timeout = Duration::from_secs(config.poll_timeout_secs);

    loop {
        tokio::select! {
            // 检查停止信号
            _ = &mut stop_rx => {
                info!("Stopping config watcher");
                break;
            }

            // 等待事件
            result = {
                let rx = rx.clone();
                tokio::task::spawn_blocking(move || {
                    let rx = rx.lock().unwrap(); // 获取互斥锁
                    rx.recv_timeout(poll_timeout)
                })
            } => {
                match result {
                    Ok(recv_result) => {
                        match recv_result {
                            Ok(event_result) => {
                                match event_result {
                                    Ok(event) => {
                                        if is_relevant_event(&event.kind) {
                                            let now = Instant::now();
                                            if now.duration_since(last_event_time) > debounce_duration {
                                                info!("Config file event: {:?}", event);
                                                handle_config_change(
                                                    &config_path,
                                                    watcher.clone(),
                                                    callback.clone(),
                                                    &config,
                                                    event.kind
                                                ).await;
                                                last_event_time = now;
                                            }
                                        }
                                    },
                                    Err(e) => error!("Watch error: {:?}", e),
                                }
                            },
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                error!("Watcher channel disconnected");
                                break;
                            },
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                                // 超时，继续循环检查停止信号
                                continue;
                            },
                        }
                    },
                    Err(e) => error!("Task spawn error: {:?}", e),
                }
            }
        }
    }

    // 停止 watch
    if let Err(e) = watcher.lock().unwrap().unwatch(&config_path) {
        error!("Failed to unwatch config file: {:?}", e);
    }

    Ok(())
}

/// 判断事件是否与配置文件变更相关
fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(notify::event::ModifyKind::Data(_)) // 文件内容变更
        | EventKind::Modify(notify::event::ModifyKind::Name(_)) // 文件重命名
        | EventKind::Remove(_) // 文件删除
        | EventKind::Create(_) // 文件创建
    )
}

/// 判断是否需要重新 watch 文件
fn needs_re_watch(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Remove(_) | EventKind::Modify(notify::event::ModifyKind::Name(_))
    )
}

/// 处理配置文件变更
async fn handle_config_change(
    config_path: &std::path::Path,
    watcher: std::sync::Arc<std::sync::Mutex<Box<dyn Watcher + Send>>>,
    callback: std::sync::Arc<ConfigChangeCallback>,
    config: &ConfigWatcherConfig,
    event_kind: EventKind,
) {
    let needs_re_watch = needs_re_watch(event_kind);

    // 对于文件重命名/覆盖事件，先等待一小段时间确保文件写入完成
    if needs_re_watch {
        time::sleep(Duration::from_millis(config.rewatch_delay_ms)).await;
    }

    // 重新读取配置文件，添加重试机制
    let result = match config_path.to_str() {
        Some(config_str) => {
            retry_with_delay(
                config.max_retries,
                Duration::from_millis(config.retry_delay_ms),
                || Settings::new(config_str),
            )
            .await
        }
        None => Err(crate::error::Error::Any(anyhow::anyhow!(
            "Config path is not valid UTF-8"
        ))),
    };

    // 如果需要重新 watch 文件，使用 spawn_blocking 避免 MutexGuard 跨 await
    if needs_re_watch {
        let watcher = watcher.clone();
        let config_path = config_path.to_path_buf();
        let config = config.clone();

        tokio::task::spawn_blocking(move || {
            let mut watcher = watcher.lock().unwrap();
            if let Err(e) = watcher.unwatch(&config_path) {
                error!("Failed to unwatch config file (ignored): {:?}", e);
            }

            // 重试机制：文件可能短暂不存在
            let mut retry_count = 0;
            let retry_delay = std::time::Duration::from_millis(config.retry_delay_ms);

            while retry_count < config.max_retries {
                match watcher.watch(&config_path, RecursiveMode::NonRecursive) {
                    Ok(_) => {
                        info!("Re-watching config file: {:?}", config_path);
                        return;
                    }
                    Err(e) => {
                        error!(
                            "Failed to re-watch config file (retry {}): {:?}",
                            retry_count + 1,
                            e
                        );
                        retry_count += 1;
                        std::thread::sleep(retry_delay);
                    }
                }
            }

            error!(
                "Failed to re-watch config file after {} retries",
                config.max_retries
            );
        })
        .await
        .ok(); // 忽略 join 错误
    }

    callback(result).await;
}

/// 通用重试函数
async fn retry_with_delay<T, E, F>(
    max_retries: usize,
    delay: Duration,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Debug,
{
    let mut attempt = 0;

    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_retries => {
                error!("Operation failed (retry {}): {:?}", attempt + 1, e);
                attempt += 1;
                time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
