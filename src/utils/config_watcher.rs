use notify::{EventKind, RecursiveMode, Watcher};
use std::{path::Path, sync::mpsc, time::Duration};
use tracing::{error, info};

use crate::config::Settings;
use crate::error::Result;

/// 启动配置文件监听
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
    + 'static,
) -> Result<mpsc::Sender<()>, notify::Error> {
    let (stop_tx, stop_rx) = mpsc::channel();
    let config_path = config_path.as_ref().to_owned();

    // 使用 tokio::spawn 代替 std::thread::spawn，确保回调在 Tokio 运行时中执行
    tokio::spawn(async move {
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
                    // 过滤只处理表示文件内容真正变更的事件类型，忽略访问事件和元数据修改事件
                    let is_relevant_event = matches!(
                        event.kind,
                        EventKind::Modify(notify::event::ModifyKind::Data(_)) // 文件内容变更
                        | EventKind::Modify(notify::event::ModifyKind::Name(_)) // 文件重命名
                        | EventKind::Remove(_) // 文件删除
                        | EventKind::Create(_) // 文件创建
                    );

                    if !is_relevant_event {
                        // 忽略不相关的事件，如访问事件、元数据修改事件等
                        continue;
                    }

                    let now = std::time::Instant::now();
                    if now.duration_since(last_event_time) > debounce_duration {
                        info!("Config file event: {:?}", event);

                        // 处理文件删除/覆盖导致 watch 失效的问题
                        // 当文件被删除、重命名或属性改变时，可能需要重新 watch
                        let needs_re_watch = matches!(
                            event.kind,
                            EventKind::Remove(_)
                                | EventKind::Modify(notify::event::ModifyKind::Name(_))
                        );

                        // 对于文件重命名/覆盖事件，先等待一小段时间确保文件写入完成
                        if needs_re_watch {
                            std::thread::sleep(Duration::from_millis(800));
                        }

                        // 重新读取配置文件，添加重试机制
                        let config_str = config_path
                            .to_str()
                            .expect("Config path is not valid UTF-8");

                        let mut retry_count = 0;
                        let max_retries = 5;
                        let retry_delay = Duration::from_millis(100);
                        let result = loop {
                            let attempt = Settings::new(config_str);
                            match attempt {
                                Ok(settings) => break Ok(settings),
                                Err(e) => {
                                    if retry_count < max_retries {
                                        error!(
                                            "Failed to read config file (retry {}): {:?}",
                                            retry_count + 1,
                                            e
                                        );
                                        retry_count += 1;
                                        std::thread::sleep(retry_delay);
                                    } else {
                                        break Err(e);
                                    }
                                }
                            }
                        };

                        // 如果需要重新 watch 文件（在读取配置成功后）
                        if needs_re_watch {
                            if let Err(e) = watcher.unwatch(&config_path) {
                                error!("Failed to unwatch config file (ignored): {:?}", e);
                            }
                            // 重试机制：文件可能短暂不存在，最多重试 5 次，每次间隔 100ms
                            let mut retry_count = 0;
                            let max_retries = 5;
                            let mut watch_successful = false;

                            while retry_count < max_retries {
                                match watcher.watch(&config_path, RecursiveMode::NonRecursive) {
                                    Ok(_) => {
                                        info!("Re-watching config file: {:?}", config_path);
                                        watch_successful = true;
                                        break;
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

                            if !watch_successful {
                                error!(
                                    "Failed to re-watch config file after {} retries",
                                    max_retries
                                );
                            }
                        }

                        callback(result).await;

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
