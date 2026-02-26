//! 静态文件服务集成测试

use std::fs;

use anyhow::Result;
use serial_test::serial;
use tempfile::TempDir;

mod common;
use common::*;

#[tokio::test]
#[serial]
async fn test_static_file_serving() -> Result<()> {
    // 创建临时目录和测试文件
    let temp_dir = TempDir::new()?;
    let test_file_path = temp_dir.path().join("index.html");
    fs::write(&test_file_path, "<html><body>Test Page</body></html>")?;

    // 创建测试配置
    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            index: Some(vec!["index.html".to_string()]),
            auto_index: None,
            upstream: None,
            redirect: None,
        }],
        ..TestServerConfig::default()
    };

    let config_path = create_temp_config(&config)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    // 发送请求到服务器
    let response = send_test_request(server_addr, "/").await?;

    // 验证响应
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.text().await?;
    assert!(body.contains("Test Page"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_directory_listing() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // 创建测试文件
    fs::write(temp_dir.path().join("file1.txt"), "Content 1")?;
    fs::write(temp_dir.path().join("file2.txt"), "Content 2")?;

    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            index: None,
            auto_index: Some(true),
            upstream: None,
            redirect: None,
        }],
        ..TestServerConfig::default()
    };

    let config_path = create_temp_config(&config)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let response = send_test_request(server_addr, "/").await?;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.text().await?;
    assert!(body.contains("file1.txt"));
    assert!(body.contains("file2.txt"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_file_not_found() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            index: None,
            auto_index: Some(true),
            upstream: None,
            redirect: None,
        }],
        ..TestServerConfig::default()
    };

    let config_path = create_temp_config(&config)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let response = send_test_request(server_addr, "/nonexistent").await?;

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_custom_error_page() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let error_page_path = temp_dir.path().join("404.html");
    fs::write(&error_page_path, "<html><body>Custom 404</body></html>")?;

    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            index: None,
            auto_index: Some(true),
            upstream: None,
            redirect: None,
        }],
        error_pages: vec![TestErrorPage {
            status: 404,
            page: "/404.html".to_string(),
        }],
        ..TestServerConfig::default()
    };

    let config_path = create_temp_config(&config)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let response = send_test_request(server_addr, "/nonexistent").await?;

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await?;
    assert!(body.contains("Custom 404"));

    Ok(())
}
