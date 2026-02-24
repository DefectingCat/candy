//! 配置生成测试

use anyhow::Result;

mod common;
use common::*;

#[tokio::test]
async fn test_config_generation() -> Result<()> {
    // 创建临时目录和测试文件
    let temp_dir = tempfile::TempDir::new()?;
    let test_file_path = temp_dir.path().join("index.html");
    std::fs::write(&test_file_path, "<html><body>Test Page</body></html>")?;

    // 创建测试配置
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

    println!("Generated config path: {}", config_path.display());

    // 验证配置文件内容
    let config_content = std::fs::read_to_string(&config_path)?;
    println!("Generated config:\n{}", config_content);

    assert!(config_content.contains(&config.ip));
    assert!(config_content.contains(&config.port.to_string()));
    assert!(config_content.contains(&format!("ssl = {}", config.ssl)));
    assert!(config_content.contains("location = \"/\""));
    assert!(config_content.contains(&format!("root = \"{}\"", temp_dir.path().display())));
    assert!(config_content.contains("index = [\"index.html\"]"));

    Ok(())
}

#[tokio::test]
async fn test_config_with_error_page() -> Result<()> {
    let temp_dir = tempfile::TempDir::new()?;

    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            index: Some(vec!["index.html".to_string()]),
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
    let config_content = std::fs::read_to_string(&config_path)?;

    assert!(config_content.contains("error_page"));
    assert!(config_content.contains("status = 404"));
    assert!(config_content.contains("page = \"/404.html\""));

    Ok(())
}
