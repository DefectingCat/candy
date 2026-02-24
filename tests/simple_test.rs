//! 简单的集成测试，用于验证测试架构

use anyhow::Result;

mod common;
use common::*;

#[tokio::test]
async fn test_simple_request() -> Result<()> {
    // 创建临时配置
    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(std::path::PathBuf::from("/tmp")), // 使用临时目录
            index: Some(vec!["index.html".to_string()]),
            auto_index: Some(false),
            upstream: None,
            redirect: None,
        }],
        ..TestServerConfig::default()
    };

    let config_path = create_temp_config(&config)?;
    println!("测试配置路径: {:?}", config_path);

    Ok(())
}
