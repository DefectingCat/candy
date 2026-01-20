use notify::{EventKind, RecursiveMode, Watcher};
use std::{path::Path, sync::mpsc, time::Duration};
use tracing::{error, info};

/// 启动配置文件监听
///
/// # 参数
///
/// * `config_path` - 配置文件路径
/// * `callback` - 配置文件变化时的回调函数
///
/// # 返回值
///
/// 返回一个发送器，用于发送停止信号
pub fn start_config_watcher(
    config_path: impl AsRef<Path>,
    callback: impl Fn() + Send + 'static,
) -> Result<mpsc::Sender<()>, notify::Error> {
    let (stop_tx, stop_rx) = mpsc::channel();
    let config_path = config_path.as_ref().to_owned();

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create watcher: {:?}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(&config_path, RecursiveMode::NonRecursive) {
            error!(
                "Failed to watch config file: {:?}, error: {:?}",
                config_path, e
            );
            return;
        }

        info!("Watching config file: {:?}", config_path);

        let mut last_event_time = std::time::Instant::now();
        let debounce_duration = Duration::from_millis(500);

        loop {
            // 检查是否有停止信号
            if stop_rx.try_recv().is_ok() {
                info!("Stopping config watcher");
                break;
            }

            // 等待事件，带超时
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Ok(event)) => {
                    let now = std::time::Instant::now();
                    if now.duration_since(last_event_time) > debounce_duration {
                        info!("Config file event: {:?}", event);

                        // 处理文件删除/覆盖导致 watch 失效的问题
                        // 当文件被删除、重命名或属性改变时，可能需要重新 watch
                        match event.kind {
                            EventKind::Remove(_)
                            | EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
                                // 重新添加 watch，处理文件被覆盖或删除的情况
                                if let Err(e) = watcher.unwatch(&config_path) {
                                    error!("Failed to unwatch config file (ignored): {:?}", e);
                                }
                                if let Err(e) =
                                    watcher.watch(&config_path, RecursiveMode::NonRecursive)
                                {
                                    error!("Failed to re-watch config file: {:?}", e);
                                } else {
                                    info!("Re-watching config file: {:?}", config_path);
                                }
                            }
                            _ => {}
                        }

                        callback();
                        last_event_time = now;
                    }
                }
                Ok(Err(e)) => error!("Watch error: {:?}", e),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    error!("Watcher channel disconnected");
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 超时，继续循环检查停止信号
                    continue;
                }
            }
        }

        if let Err(e) = watcher.unwatch(&config_path) {
            error!("Failed to unwatch config file: {:?}", e);
        }
    });

    Ok(stop_tx)
}
