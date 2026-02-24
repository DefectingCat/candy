//! 简单的服务器启动测试

use anyhow::Result;

mod common;
use common::*;

#[tokio::test]
async fn test_simple_startup() -> Result<()> {
    println!("Starting simple server startup test...");
    
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
    
    println!("Generated config path: {}", config_path.display());
    
    let server_handle = start_test_server(&config_path).await?;
    
    println!("Server handle created successfully");
    
    Ok(())
}