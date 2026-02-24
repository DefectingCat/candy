//! 集成测试的公共辅助函数和工具

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
            port: 59999, // 使用固定测试端口
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
pub async fn start_test_server(config_path: &PathBuf) -> Result<axum_server::Handle<SocketAddr>> {
    let _ = logging::init_logger("debug", "/dev/null").expect("Failed to init logger");

    let settings = Settings::new(config_path.to_str().expect("Invalid path")).expect("Failed to load config");

    let server_handle = http::make_server(settings.host.into_iter().next().expect("No host config"))
        .await
        .expect("Failed to create server");

    Ok(server_handle)
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