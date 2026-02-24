//! 基本服务测试

use anyhow::Result;

mod common;
use common::*;

mod test_fixtures;
use test_fixtures::*;

#[tokio::test]
async fn test_basic_server() -> Result<()> {
    println!("测试基本服务器功能...");
    
    let (temp_dir_path, _index_html_path) = create_test_directory()?;
    
    let config = TestServerConfig {
        routes: vec![TestRoute {
            location: "/".to_string(),
            root: Some(temp_dir_path.clone()),
            index: Some(vec!["index.html".to_string()]),
            auto_index: Some(false), // 禁用自动索引
            upstream: None,
            redirect: None,
        }],
        ..TestServerConfig::default()
    };
    
    let config_path = create_temp_config(&config)?;
    let server_handle = start_test_server(&config_path).await?;
    
    // 获取服务器地址
    let server_addr = get_server_addr(&server_handle).await;
    println!("服务器地址: {}", server_addr);
    
    // 发送测试请求
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    
    let url = format!("http://{}", server_addr);
    println!("测试 URL: {}", url);
    
    let response = client.get(&url).send().await?;
    
    println!("响应状态码: {}", response.status());
    assert!(response.status().is_success());
    
    let response_text = response.text().await?;
    println!("响应内容: {}", response_text);
    assert!(response_text.contains("Test Page"));
    
    println!("服务器功能测试通过!");
    Ok(())
}