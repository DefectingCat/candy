use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Duration, Instant};
use tracing::{debug, error, info};

use crate::config::Settings;
use crate::error::Result;

/// 配置变更回调函数类型（使用 BoxFuture 简化）
pub type ConfigChangeCallback =
    dyn Fn(Result<Settings>) -> futures::future::BoxFuture<'static, ()> + Send + Sync + 'static;

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

/// 启动配置文件监听（使用默认配置）
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
    callback: impl Fn(Result<Settings>) -> futures::future::BoxFuture<'static, ()>
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
/// * `watcher_config` - 监听器配置参数（可选，使用默认值）
///
/// # 返回值
///
/// 返回一个发送器，用于发送停止信号
pub fn start_config_watcher_with_config(
    config_path: impl AsRef<Path>,
    callback: impl Fn(Result<Settings>) -> futures::future::BoxFuture<'static, ()>
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
///
/// # 参数
///
/// * `config_path` - 配置文件路径
/// * `callback` - 配置文件变化时的回调函数，参数为重新读取后的配置
/// * `config` - 监听器配置参数
/// * `stop_rx` - 停止信号接收端
///
/// # 返回值
///
/// 返回操作结果，成功或包含错误信息
async fn run_watcher(
    config_path: std::path::PathBuf,
    callback: std::sync::Arc<ConfigChangeCallback>,
    config: ConfigWatcherConfig,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<(), notify::Error> {
    let (event_tx, mut event_rx) = mpsc::channel(10);
    let watcher = std::sync::Arc::new(std::sync::Mutex::new(Box::new(notify::recommended_watcher(
        move |res| {
            let _ = event_tx.try_send(res);
        },
    )?) as Box<dyn Watcher + Send>));

    watcher
        .lock()
        .map_err(|e| {
            let msg = format!("Failed to lock watcher mutex: {:?}", e);
            error!("{}", msg);
            notify::Error::generic(&msg)
        })?
        .watch(&config_path, RecursiveMode::NonRecursive)?;

    info!("Watching config file: {:?}", config_path);

    let mut last_event_time = Instant::now();
    let debounce_duration = Duration::from_millis(config.debounce_ms);
    let poll_timeout = Duration::from_secs(config.poll_timeout_secs);
    let is_processing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                info!("Stopping config watcher");
                break;
            }

            result = event_rx.recv() => {
                if let Err(e) = process_event(
                    result,
                    EventProcessingContext {
                        is_processing: &is_processing,
                        last_event_time: &mut last_event_time,
                        debounce_duration,
                        config_path: &config_path,
                        watcher: &watcher,
                        callback: &callback,
                        config: &config,
                    },
                ).await {
                    error!("Event processing failed: {:?}", e);
                }
            }

            _ = time::sleep(poll_timeout) => continue,
        }
    }

    if let Ok(mut w) = watcher.lock() {
        let _ = w.unwatch(&config_path);
    } else {
        error!("Failed to lock watcher mutex for unwatch");
    }

    Ok(())
}

/// 处理单个配置文件事件的上下文结构体
struct EventProcessingContext<'a> {
    is_processing: &'a std::sync::Arc<std::sync::atomic::AtomicBool>,
    last_event_time: &'a mut Instant,
    debounce_duration: Duration,
    config_path: &'a std::path::Path,
    watcher: &'a std::sync::Arc<std::sync::Mutex<Box<dyn Watcher + Send>>>,
    callback: &'a std::sync::Arc<ConfigChangeCallback>,
    config: &'a ConfigWatcherConfig,
}

/// 处理单个配置文件事件
///
/// # 参数
///
/// * `result` - 通知库返回的事件结果（可能包含错误）
/// * `ctx` - 事件处理上下文
///
/// # 返回值
///
/// 返回操作结果，成功或包含错误信息
async fn process_event(
    result: Option<std::result::Result<notify::Event, notify::Error>>,
    ctx: EventProcessingContext<'_>,
) -> Result<(), notify::Error> {
    match result {
        Some(event_result) => match event_result {
            Ok(event) => {
                if is_relevant_event(&event.kind) {
                    let now = Instant::now();
                    let processing_flag =
                        ctx.is_processing.load(std::sync::atomic::Ordering::Relaxed);

                    if now.duration_since(*ctx.last_event_time) > ctx.debounce_duration
                        && !processing_flag
                    {
                        info!("Config file event: {:?}", event);
                        ctx.is_processing
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        *ctx.last_event_time = now;

                        let config_path_clone = ctx.config_path.to_path_buf();
                        let watcher_clone = ctx.watcher.clone();
                        let callback_clone = ctx.callback.clone();
                        let config_clone = ctx.config.clone();
                        let event_kind_clone = event.kind;
                        let is_processing_clone = ctx.is_processing.clone();
                        let debounce_duration_clone = ctx.debounce_duration;

                        tokio::spawn(async move {
                            handle_config_change(
                                &config_path_clone,
                                watcher_clone,
                                callback_clone,
                                &config_clone,
                                event_kind_clone,
                            )
                            .await;

                            time::sleep(debounce_duration_clone).await;
                            is_processing_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                        });
                    } else {
                        debug!("Ignoring duplicate event within debounce window");
                    }
                }
            }
            Err(e) => error!("Watch error: {:?}", e),
        },
        None => {
            error!("Watcher channel disconnected");
            return Err(notify::Error::generic("Watcher channel disconnected"));
        }
    }

    Ok(())
}

/// 判断事件是否与配置文件变更相关
///
/// # 参数
///
/// * `kind` - 通知库返回的事件类型
///
/// # 返回值
///
/// 返回事件是否与配置文件变更相关
fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(notify::event::ModifyKind::Data(_))
            | EventKind::Modify(notify::event::ModifyKind::Name(_))
            | EventKind::Remove(_)
            | EventKind::Create(_)
    )
}

/// 判断是否需要重新 watch 文件
///
/// # 参数
///
/// * `kind` - 通知库返回的事件类型
///
/// # 返回值
///
/// 返回是否需要重新 watch 文件
fn needs_re_watch(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Remove(_) | EventKind::Modify(notify::event::ModifyKind::Name(_))
    )
}

/// 处理配置文件变更
///
/// # 参数
///
/// * `config_path` - 配置文件路径
/// * `watcher` - 配置文件监听器实例
/// * `callback` - 配置变化时的回调函数
/// * `config` - 监听器配置参数
/// * `event_kind` - 触发配置变更的事件类型
async fn handle_config_change(
    config_path: &std::path::Path,
    watcher: std::sync::Arc<std::sync::Mutex<Box<dyn Watcher + Send>>>,
    callback: std::sync::Arc<ConfigChangeCallback>,
    config: &ConfigWatcherConfig,
    event_kind: EventKind,
) {
    let needs_re_watch_flag = needs_re_watch(event_kind);

    if needs_re_watch_flag {
        time::sleep(Duration::from_millis(config.rewatch_delay_ms)).await;
    }

    let config_result = match config_path.to_str() {
        Some(config_str) => {
            retry_operation(
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

    if needs_re_watch_flag {
        let watcher_clone = watcher.clone();
        let config_path_clone = config_path.to_path_buf();
        let config_clone = config.clone();

        if let Err(e) = tokio::task::spawn_blocking(move || {
            retry_sync_operation(
                config_clone.max_retries,
                std::time::Duration::from_millis(config_clone.retry_delay_ms),
                || {
                    let mut w = watcher_clone.lock().map_err(|e| {
                        let msg = format!("Failed to lock watcher mutex: {:?}", e);
                        notify::Error::generic(&msg)
                    })?;

                    let _ = w.unwatch(&config_path_clone);
                    w.watch(&config_path_clone, RecursiveMode::NonRecursive)
                },
            )
        })
        .await
        {
            error!("Failed to join re-watch task: {:?}", e);
        } else {
            info!("Re-watching config file: {:?}", config_path);
        }
    }

    callback(config_result).await;
}

/// 异步重试操作
///
/// # 参数
///
/// * `max_retries` - 最大重试次数
/// * `delay` - 重试间隔
/// * `operation` - 需要重试的操作
///
/// # 类型参数
///
/// * `T` - 操作成功时返回的类型
/// * `E` - 操作失败时返回的错误类型
/// * `F` - 操作函数类型，返回 Result<T, E>
///
/// # 返回值
///
/// 返回操作结果，成功或包含错误信息
async fn retry_operation<T, E, F>(
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

/// 同步重试操作
///
/// # 参数
///
/// * `max_retries` - 最大重试次数
/// * `delay` - 重试间隔
/// * `operation` - 需要重试的操作
///
/// # 类型参数
///
/// * `T` - 操作成功时返回的类型
/// * `E` - 操作失败时返回的错误类型
/// * `F` - 操作函数类型，返回 Result<T, E>
///
/// # 返回值
///
/// 返回操作结果，成功或包含错误信息
fn retry_sync_operation<T, E, F>(
    max_retries: usize,
    delay: std::time::Duration,
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
                std::thread::sleep(delay);
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::EventKind;

    #[test]
    fn test_relevant_events() {
        assert!(is_relevant_event(&EventKind::Modify(
            notify::event::ModifyKind::Data(notify::event::DataChange::Content)
        )));
        assert!(is_relevant_event(&EventKind::Modify(
            notify::event::ModifyKind::Name(notify::event::RenameMode::To)
        )));
        assert!(is_relevant_event(&EventKind::Remove(
            notify::event::RemoveKind::File
        )));
        assert!(is_relevant_event(&EventKind::Create(
            notify::event::CreateKind::File
        )));

        assert!(!is_relevant_event(&EventKind::Access(
            notify::event::AccessKind::Close(notify::event::AccessMode::Write)
        )));
        assert!(!is_relevant_event(&EventKind::Other));
    }

    #[test]
    fn test_needs_re_watch_events() {
        assert!(needs_re_watch(EventKind::Remove(
            notify::event::RemoveKind::File
        )));
        assert!(needs_re_watch(EventKind::Modify(
            notify::event::ModifyKind::Name(notify::event::RenameMode::To)
        )));

        assert!(!needs_re_watch(EventKind::Modify(
            notify::event::ModifyKind::Data(notify::event::DataChange::Content)
        )));
        assert!(!needs_re_watch(EventKind::Create(
            notify::event::CreateKind::File
        )));
        assert!(!needs_re_watch(EventKind::Other));
    }

    #[test]
    fn test_default_watcher_config() {
        let default_config = ConfigWatcherConfig::default();
        assert_eq!(default_config.debounce_ms, 500);
        assert_eq!(default_config.rewatch_delay_ms, 800);
        assert_eq!(default_config.max_retries, 5);
        assert_eq!(default_config.retry_delay_ms, 100);
        assert_eq!(default_config.poll_timeout_secs, 1);
    }
}
