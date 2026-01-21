use std::time::Duration;
use tokio::time::sleep;

use candy::config::Settings;
use candy::utils::config_watcher::{self, ConfigWatcherConfig};

#[tokio::main]
async fn main() {
    println!("Testing config watcher...");

    // 创建一个临时配置文件用于测试
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 写入初始配置
    std::fs::write(&config_path, r#"
[server]
port = 8080
host = "127.0.0.1"

[log]
level = "info"
folder = "logs"
    "#).unwrap();

    println!("Config file created at: {:?}", config_path);

    // 测试 start_config_watcher（简化版本）
    println!("\n1. Testing start_config_watcher (default config)...");
    let stop_tx1 = config_watcher::start_config_watcher(
        &config_path,
        move |result| {
            Box::pin(async move {
                println!("Callback 1 received result: {:?}", result);
            })
        }
    ).unwrap();

    // 测试 start_config_watcher_with_config（自定义配置）
    println!("\n2. Testing start_config_watcher_with_config (custom config)...");
    let custom_config = ConfigWatcherConfig {
        debounce_ms: 200,
        rewatch_delay_ms: 500,
        max_retries: 3,
        retry_delay_ms: 50,
        poll_timeout_secs: 2,
    };

    let stop_tx2 = config_watcher::start_config_watcher_with_config(
        &config_path,
        move |result| {
            Box::pin(async move {
                println!("Callback 2 received result: {:?}", result);
            })
        },
        Some(custom_config)
    ).unwrap();

    // 等待一段时间让 watcher 启动
    sleep(Duration::from_millis(500)).await;

    // 修改配置文件以触发事件
    println!("\n3. Modifying config file to trigger change...");
    std::fs::write(&config_path, r#"
[server]
port = 9090
host = "0.0.0.0"

[log]
level = "debug"
folder = "debug_logs"
    "#).unwrap();

    // 等待事件被处理
    sleep(Duration::from_secs(2)).await;

    // 停止 watcher
    println!("\n4. Stopping watchers...");
    stop_tx1.send(()).unwrap();
    stop_tx2.send(()).unwrap();

    // 等待一段时间让 watcher 停止
    sleep(Duration::from_millis(500)).await;

    println!("\n✅ Test completed successfully!");
}
