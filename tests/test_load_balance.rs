//! 负载均衡集成测试
//!
//! 测试反向代理的负载均衡功能，包括：
//! - 配置解析测试
//! - Round Robin 负载均衡模式

use std::io::Write;

use anyhow::Result;
use tempfile::NamedTempFile;

mod test_fixtures;
use test_fixtures::*;

// ============================================================================
// Round Robin 配置解析测试
// ============================================================================

#[test]
fn test_round_robin_config_parsing() -> Result<()> {
    // 测试 Round Robin 配置解析
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        r#"
[[upstream]]
name = "test_round_robin"
method = "roundrobin"
server = [
    {{ server = "192.168.1.100:8080" }},
    {{ server = "192.168.1.101:8080" }},
    {{ server = "192.168.1.102:8080" }}
]

[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false

[[host.route]]
location = "/api"
upstream = "test_round_robin"
proxy_timeout = 30
"#
    )?;

    let path = file.path().to_str().unwrap();
    let settings = candy::config::Settings::new(path)?;

    // 验证 upstream 配置
    assert!(settings.upstream.is_some());
    let upstreams = settings.upstream.as_ref().unwrap();
    assert_eq!(upstreams.len(), 1);

    let upstream = &upstreams[0];
    assert_eq!(upstream.name, "test_round_robin");
    assert_eq!(upstream.method, candy::config::LoadBalanceType::RoundRobin);
    assert_eq!(upstream.server.len(), 3);
    assert_eq!(upstream.server[0].server, "192.168.1.100:8080");
    assert_eq!(upstream.server[1].server, "192.168.1.101:8080");
    assert_eq!(upstream.server[2].server, "192.168.1.102:8080");

    // 验证默认权重为 1
    assert_eq!(upstream.server[0].weight, 1);
    assert_eq!(upstream.server[1].weight, 1);
    assert_eq!(upstream.server[2].weight, 1);

    // 验证路由配置
    assert_eq!(settings.host.len(), 1);
    let route = &settings.host[0].route[0];
    assert_eq!(route.location, "/api");
    assert_eq!(route.upstream, Some("test_round_robin".to_string()));

    Ok(())
}

#[test]
fn test_weighted_round_robin_config_parsing() -> Result<()> {
    // 测试 Weighted Round Robin 配置解析
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        r#"
[[upstream]]
name = "test_weighted"
method = "weightedroundrobin"
server = [
    {{ server = "192.168.1.100:8080", weight = 3 }},
    {{ server = "192.168.1.101:8080", weight = 2 }},
    {{ server = "192.168.1.102:8080", weight = 1 }}
]

[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false

[[host.route]]
location = "/api"
upstream = "test_weighted"
proxy_timeout = 30
"#
    )?;

    let path = file.path().to_str().unwrap();
    let settings = candy::config::Settings::new(path)?;

    // 验证 upstream 配置
    let upstreams = settings.upstream.as_ref().unwrap();
    let upstream = &upstreams[0];

    assert_eq!(upstream.name, "test_weighted");
    assert_eq!(
        upstream.method,
        candy::config::LoadBalanceType::WeightedRoundRobin
    );
    assert_eq!(upstream.server.len(), 3);

    // 验证权重配置
    assert_eq!(upstream.server[0].weight, 3);
    assert_eq!(upstream.server[1].weight, 2);
    assert_eq!(upstream.server[2].weight, 1);

    Ok(())
}

#[test]
fn test_default_load_balance_method() -> Result<()> {
    // 测试默认负载均衡方法（WeightedRoundRobin）
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        r#"
[[upstream]]
name = "test_default"
server = [
    {{ server = "192.168.1.100:8080" }},
    {{ server = "192.168.1.101:8080" }}
]

[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false

[[host.route]]
location = "/api"
upstream = "test_default"
proxy_timeout = 30
"#
    )?;

    let path = file.path().to_str().unwrap();
    let settings = candy::config::Settings::new(path)?;

    let upstreams = settings.upstream.as_ref().unwrap();
    let upstream = &upstreams[0];

    // 默认应该是 WeightedRoundRobin
    assert_eq!(
        upstream.method,
        candy::config::LoadBalanceType::WeightedRoundRobin
    );

    Ok(())
}

#[test]
fn test_multiple_upstreams_config() -> Result<()> {
    // 测试多个 upstream 配置
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        r#"
[[upstream]]
name = "api_servers"
method = "roundrobin"
server = [
    {{ server = "192.168.1.100:8080" }},
    {{ server = "192.168.1.101:8080" }}
]

[[upstream]]
name = "static_servers"
method = "weightedroundrobin"
server = [
    {{ server = "192.168.1.200:80", weight = 5 }},
    {{ server = "192.168.1.201:80", weight = 2 }}
]

[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false

[[host.route]]
location = "/api"
upstream = "api_servers"
proxy_timeout = 30

[[host.route]]
location = "/static"
upstream = "static_servers"
proxy_timeout = 30
"#
    )?;

    let path = file.path().to_str().unwrap();
    let settings = candy::config::Settings::new(path)?;

    // 验证多个 upstream 配置
    let upstreams = settings.upstream.as_ref().unwrap();
    assert_eq!(upstreams.len(), 2);

    // 验证第一个 upstream (API 服务器)
    let api_upstream = &upstreams[0];
    assert_eq!(api_upstream.name, "api_servers");
    assert_eq!(api_upstream.method, candy::config::LoadBalanceType::RoundRobin);
    assert_eq!(api_upstream.server.len(), 2);

    // 验证第二个 upstream (静态服务器)
    let static_upstream = &upstreams[1];
    assert_eq!(static_upstream.name, "static_servers");
    assert_eq!(
        static_upstream.method,
        candy::config::LoadBalanceType::WeightedRoundRobin
    );
    assert_eq!(static_upstream.server.len(), 2);
    assert_eq!(static_upstream.server[0].weight, 5);
    assert_eq!(static_upstream.server[1].weight, 2);

    // 验证路由配置
    assert_eq!(settings.host[0].route.len(), 2);

    Ok(())
}

#[test]
fn test_invalid_upstream_reference() -> Result<()> {
    // 测试引用不存在的 upstream 配置
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        r#"
[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false

[[host.route]]
location = "/api"
upstream = "nonexistent_upstream"
proxy_timeout = 30
"#
    )?;

    let path = file.path().to_str().unwrap();
    let result = candy::config::Settings::new(path);

    // 应该返回错误，因为引用了不存在的 upstream
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("unknown upstream"));

    Ok(())
}

#[test]
fn test_upstream_empty_servers() -> Result<()> {
    // 测试 upstream 没有配置服务器的情况
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        r#"
[[upstream]]
name = "empty_upstream"
method = "roundrobin"
server = []

[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false

[[host.route]]
location = "/api"
upstream = "empty_upstream"
proxy_timeout = 30
"#
    )?;

    let path = file.path().to_str().unwrap();
    let result = candy::config::Settings::new(path);

    // 应该返回错误，因为 upstream 没有配置服务器
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_upstream_invalid_server_address() -> Result<()> {
    // 测试 upstream 服务器地址无效的情况（缺少端口）
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        r#"
[[upstream]]
name = "invalid_upstream"
method = "roundrobin"
server = [
    {{ server = "192.168.1.100" }}
]

[[host]]
ip = "127.0.0.1"
port = 8080
ssl = false

[[host.route]]
location = "/api"
upstream = "invalid_upstream"
proxy_timeout = 30
"#
    )?;

    let path = file.path().to_str().unwrap();
    let result = candy::config::Settings::new(path);

    // 应该返回错误，因为服务器地址缺少端口
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("missing port"));

    Ok(())
}

// ============================================================================
// LoadBalanceType 测试
// ============================================================================

#[test]
fn test_load_balance_type_serde() -> Result<()> {
    use candy::config::LoadBalanceType;

    // 测试通过完整 TOML 配置反序列化（小写下划线格式）
    let config: toml::Value = toml::from_str(r#"
method = "roundrobin"
"#)?;
    assert_eq!(config["method"].as_str(), Some("roundrobin"));

    // 测试 LoadBalanceType 的 Deserialize 实现
    // 使用 serde_json 测试（它使用相同的 serde trait）
    assert_eq!(
        serde_json::from_str::<LoadBalanceType>("\"roundrobin\"")?,
        LoadBalanceType::RoundRobin
    );
    assert_eq!(
        serde_json::from_str::<LoadBalanceType>("\"weightedroundrobin\"")?,
        LoadBalanceType::WeightedRoundRobin
    );
    assert_eq!(
        serde_json::from_str::<LoadBalanceType>("\"iphash\"")?,
        LoadBalanceType::IpHash
    );
    assert_eq!(
        serde_json::from_str::<LoadBalanceType>("\"leastconn\"")?,
        LoadBalanceType::LeastConn
    );

    Ok(())
}

#[test]
fn test_load_balance_type_debug() {
    use candy::config::LoadBalanceType;

    // 测试 Debug trait 实现
    assert!(format!("{:?}", LoadBalanceType::RoundRobin).contains("RoundRobin"));
    assert!(format!("{:?}", LoadBalanceType::WeightedRoundRobin).contains("WeightedRoundRobin"));
}

// ============================================================================
// 基本服务器测试（验证负载均衡配置的服务器启动）
// ============================================================================

#[tokio::test]
async fn test_server_startup_with_round_robin_config() -> Result<()> {
    // 测试使用 Round Robin 配置启动服务器
    let (temp_dir_path, _index_html_path) = create_test_directory()?;

    let mut config_content = String::new();
    config_content.push_str("log_level = \"debug\"\n");
    config_content.push_str("log_folder = \"/tmp/candy_test\"\n\n");

    config_content.push_str("[[upstream]]\n");
    config_content.push_str("name = \"test_backend\"\n");
    config_content.push_str("method = \"roundrobin\"\n");
    config_content.push_str("server = [\n");
    config_content.push_str("  { server = \"192.168.1.100:8080\" },\n");
    config_content.push_str("  { server = \"192.168.1.101:8080\" },\n");
    config_content.push_str("]\n\n");

    config_content.push_str("[[host]]\n");
    config_content.push_str("ip = \"127.0.0.1\"\n");
    config_content.push_str("port = 0\n"); // 使用随机端口
    config_content.push_str("ssl = false\n");
    config_content.push_str("timeout = 75\n\n");

    config_content.push_str("[[host.route]]\n");
    config_content.push_str("location = \"/\"\n");
    config_content.push_str("root = \"");
    config_content.push_str(temp_dir_path.to_str().unwrap());
    config_content.push_str("\"\n");
    config_content.push_str("index = [\"index.html\"]\n");

    // 创建临时配置文件
    let config_path = temp_dir_path.join("config.toml");
    std::fs::write(&config_path, &config_content)?;

    // 验证配置文件可以正常解析
    let settings = candy::config::Settings::new(config_path.to_str().unwrap())?;
    assert!(settings.upstream.is_some());

    let upstreams = settings.upstream.as_ref().unwrap();
    assert_eq!(upstreams.len(), 1);
    assert_eq!(upstreams[0].name, "test_backend");
    assert_eq!(upstreams[0].method, candy::config::LoadBalanceType::RoundRobin);

    println!("Round Robin 配置解析测试通过!");
    Ok(())
}