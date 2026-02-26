//! 集成测试的公共辅助函数和工具

#![allow(dead_code)] // 允许未使用的函数，这些是测试辅助函数

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use tempfile::TempDir;

use candy::config::Settings;
use candy::http;
use candy::utils::logging;

/// 测试服务器配置
#[derive(Debug)]
pub struct TestServerConfig {
    pub ip: String,
    pub port: u16,
    pub ssl: bool,
    pub routes: Vec<TestRoute>,
    pub error_pages: Vec<TestErrorPage>,
}

/// 测试路由配置
#[derive(Debug)]
pub struct TestRoute {
    pub location: String,
    pub root: Option<PathBuf>,
    pub index: Option<Vec<String>>,
    pub auto_index: Option<bool>,
    pub upstream: Option<String>,
    pub redirect: Option<String>,
}

/// 测试错误页面配置
#[derive(Debug)]
pub struct TestErrorPage {
    pub status: u16,
    pub page: String,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".to_string(),
            port: 0, // 使用随机可用端口
            ssl: false,
            routes: Vec::new(),
            error_pages: Vec::new(),
        }
    }
}

/// 创建临时配置文件用于测试
pub fn create_temp_config(config: &TestServerConfig) -> Result<PathBuf> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    // 使 temp_dir 不被自动删除（leak）
    let _ = Box::leak(Box::new(temp_dir));

    let mut config_content = String::new();
    config_content.push_str(&format!("[log]\nlevel = \"debug\"\n"));

    if !config.routes.is_empty() {
        config_content.push_str("[[host]]\n");
        config_content.push_str(&format!("ip = \"{}\"\n", config.ip));
        config_content.push_str(&format!("port = {}\n", config.port));
        config_content.push_str(&format!("ssl = {}\n", config.ssl));
        config_content.push_str(&format!("timeout = 75\n"));

        for route in &config.routes {
            config_content.push_str("[[host.route]]\n");
            config_content.push_str(&format!("location = \"{}\"\n", route.location));

            if let Some(root) = &route.root {
                config_content.push_str(&format!(
                    "root = \"{}\"\n",
                    root.to_str().expect("Invalid path")
                ));
            }

            if let Some(index) = &route.index {
                config_content.push_str(&format!(
                    "index = {:?}\n",
                    index.iter().map(|s| s.as_str()).collect::<Vec<_>>()
                ));
            }

            if let Some(auto_index) = route.auto_index {
                config_content.push_str(&format!("auto_index = {}\n", auto_index));
            }

            if let Some(upstream) = &route.upstream {
                config_content.push_str(&format!("upstream = \"{}\"\n", upstream));
            }

            if let Some(redirect) = &route.redirect {
                config_content.push_str(&format!("redirect = \"{}\"\n", redirect));
            }

            if let Some(error_page) = &config.error_pages.first() {
                config_content.push_str(&format!(
                    "error_page = {{ status = {}, page = \"{}\" }}\n",
                    error_page.status, error_page.page
                ));
            }
        }
    }

    std::fs::write(&config_path, config_content)?;
    Ok(config_path)
}

/// 启动测试服务器
pub async fn start_test_server(
    config_path: &PathBuf,
) -> Result<(axum_server::Handle<SocketAddr>, SocketAddr)> {
    // 清理全局状态，确保测试隔离
    http::clear_global_state();
    
    // 初始化 logger（幂等操作，可以多次调用）
    let _ = logging::init_logger("debug", "/dev/null");

    let settings =
        Settings::new(config_path.to_str().expect("Invalid path")).expect("Failed to load config");

    let server_handle = http::make_server(
        settings.host.into_iter().next().expect("No host config"),
        settings.compression,
    )
    .await
    .expect("Failed to create server");

    // 等待服务器开始监听（最多等待 1 秒）
    let max_wait_time = std::time::Duration::from_secs(1);
    let _start_time = std::time::Instant::now();

    // 使用超时 future 来等待服务器开始监听
    let listen_future = server_handle.listening();
    let timeout_future = tokio::time::sleep(max_wait_time);

    tokio::select! {
        addr = listen_future => {
            Ok((server_handle, addr.expect("Server failed to report listening address")))
        },
        _ = timeout_future => {
            Err(anyhow::anyhow!("Server failed to start listening within {}ms", max_wait_time.as_millis()))
        }
    }
}

/// 获取服务器实际监听地址
pub async fn get_server_addr(handle: &axum_server::Handle<SocketAddr>) -> SocketAddr {
    handle.listening().await.expect("Server not listening")
}

/// 发送HTTP请求到测试服务器
pub async fn send_test_request(addr: SocketAddr, path: &str) -> Result<reqwest::Response> {
    let client = reqwest::Client::new();
    let url = format!("http://{}{}", addr, path);

    client.get(&url).send().await.map_err(Into::into)
}
