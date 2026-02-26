//! 服务器启动测试

use anyhow::Result;
use serial_test::serial;

mod common;
use common::*;

#[tokio::test]
#[serial]
async fn test_server_startup() -> Result<()> {
    println!("Starting server startup test...");

    let temp_dir = tempfile::TempDir::new()?;
    let temp_dir_path = temp_dir.path().to_path_buf();

    let test_file_path = temp_dir_path.join("index.html");
    std::fs::write(&test_file_path, "<html><body>Test Page</body></html>")?;

    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir_path.clone()),
            index: Some(vec!["index.html".to_string()]),
            auto_index: Some(false), // 禁用自动索引，直接返回 index.html
            upstream: None,
            redirect: None,
        }],
        ..TestServerConfig::default()
    };

    let config_path = create_temp_config(&config)?;

    println!("Generated config path: {}", config_path.display());

    // 启动服务器
    let (server_handle_inner, actual_addr) = start_test_server(&config_path).await?;

    println!(
        "Server handle created successfully, listening on: {}",
        actual_addr
    );

    // 发送测试请求 - 使用超时
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let url = format!("http://{}", actual_addr);
    println!("Testing URL: {}", url);

    let response = client.get(&url).send().await;

    match response {
        Ok(res) => {
            println!("Response status code: {}", res.status());
            if res.status().is_success() {
                let body = res.text().await?;
                println!("Response body: {}", body);
                assert!(body.contains("Test Page"));
            } else {
                let status = res.status();
                let body = res.text().await?;
                println!("Response body: {}", body);
                panic!("Server returned error status: {}", status);
            }
        }
        Err(e) => {
            println!("Error sending request: {}", e);
            panic!("Failed to send request to server");
        }
    }

    // 优雅关闭服务器
    server_handle_inner.graceful_shutdown(Some(std::time::Duration::from_secs(2)));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_shutdown() -> Result<()> {
    println!("Starting server shutdown test...");

    let temp_dir = tempfile::TempDir::new()?;
    let test_file_path = temp_dir.path().join("index.html");
    std::fs::write(&test_file_path, "<html><body>Test Page</body></html>")?;

    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            index: Some(vec!["index.html".to_string()]),
            auto_index: Some(true),
            upstream: None,
            redirect: None,
        }],
        ..TestServerConfig::default()
    };

    let config_path = create_temp_config(&config)?;

    let (server_handle, _addr) = start_test_server(&config_path).await?;

    // 关闭服务器（使用优雅关闭）
    server_handle.graceful_shutdown(Some(std::time::Duration::from_secs(2)));

    // 等待服务器完全停止
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    println!("Server shutdown test completed");

    Ok(())
}
