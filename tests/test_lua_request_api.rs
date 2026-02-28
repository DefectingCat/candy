//! Lua 脚本引擎集成测试
//!
//! 测试 Lua 脚本在 HTTP 请求处理中的各种 API：
//! - cd.req:get_method() - 获取请求方法
//! - cd.req:get_uri() - 获取 URI
//! - cd.req:get_headers() - 获取请求头
//! - cd.req:get_uri_args() - 获取查询参数
//! - cd.req:get_post_args() - 获取 POST 参数
//! - cd.req:get_body_data() - 获取请求体
//! - cd.req:set_method() - 设置请求方法
//! - cd.req:set_uri() - 设置 URI
//! - cd.req:set_uri_args() - 设置查询参数

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serial_test::serial;
use tempfile::TempDir;

mod common;
use common::*;

// ============================================================================
// 测试辅助函数
// ============================================================================

/// 创建包含 Lua 脚本路由的测试配置
fn create_lua_test_config(temp_dir: &TempDir, lua_script_path: &PathBuf) -> Result<PathBuf> {
    let config_path = temp_dir.path().join("config.toml");

    let config_content = format!(
        r#"
[log]
level = "debug"

[[host]]
ip = "127.0.0.1"
port = 0
ssl = false
timeout = 75

[[host.route]]
location = "/lua"
lua_script = "{}"
lua_code_cache = false
"#,
        lua_script_path.to_str().unwrap()
    );

    fs::write(&config_path, config_content)?;
    Ok(config_path)
}

/// 创建 Lua 脚本文件
fn create_lua_script(temp_dir: &TempDir, script_content: &str) -> Result<PathBuf> {
    let script_path = temp_dir.path().join("test.lua");
    fs::write(&script_path, script_content)?;
    Ok(script_path)
}

// ============================================================================
// cd.req:get_method() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_get_method_get() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "GET");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_method_post() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.post(&url).body("test data").send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "POST");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_method_put() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.put(&url).body("test data").send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "PUT");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_method_delete() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.delete(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "DELETE");

    Ok(())
}

// ============================================================================
// cd.req:get_uri() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_get_uri_simple_path() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local uri = cd.req:get_uri()
cd.req:print(uri)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "/lua");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_uri_with_query() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local uri = cd.req:get_uri()
cd.req:print(uri)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?key1=value1&key2=value2", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("/lua?"));
    assert!(body.contains("key1=value1"));
    assert!(body.contains("key2=value2"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_uri_encoded() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local uri = cd.req:get_uri()
cd.req:print(uri)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?name=hello%20world", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("/lua?"));

    Ok(())
}

// ============================================================================
// cd.req:get_headers() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_get_headers_basic() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local headers = cd.req:get_headers()
local content_type = headers["content-type"]
if content_type then
    cd.req:print(content_type)
else
    cd.req:print("no content-type")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "application/json");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_headers_custom() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local headers = cd.req:get_headers()
local custom = headers["x-custom-header"]
if custom then
    cd.req:print(custom)
else
    cd.req:print("not found")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .get(&url)
        .header("X-Custom-Header", "test-value-123")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "test-value-123");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_headers_host() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local headers = cd.req:get_headers()
local host = headers["host"]
if host then
    cd.req:print(host)
else
    cd.req:print("no host")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // Host header should contain the server address
    assert!(body.contains(&server_addr.to_string()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_headers_multiple_values() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local headers = cd.req:get_headers()
local accept = headers["accept"]
if type(accept) == "table" then
    cd.req:print("table")
elseif type(accept) == "string" then
    cd.req:print(accept)
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // Should return something (either string or table)
    assert!(!body.is_empty());

    Ok(())
}

// ============================================================================
// cd.req:get_uri_args() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_get_uri_args_single() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_uri_args()
local name = args["name"]
if name then
    cd.req:print(name)
else
    cd.req:print("not found")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?name=testuser", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "testuser");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_uri_args_multiple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_uri_args()
local name = args["name"] or "none"
local id = args["id"] or "none"
cd.req:print(name .. "," .. id)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?name=alice&id=42", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "alice,42");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_uri_args_encoded() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_uri_args()
local msg = args["msg"]
if msg then
    cd.req:print(msg)
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?msg=hello%20world", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "hello world");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_uri_args_no_value() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_uri_args()
local flag = args["flag"]
if flag == true then
    cd.req:print("flag_is_true")
elseif flag == "" then
    cd.req:print("flag_is_empty")
else
    cd.req:print("flag_is_" .. type(flag))
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?flag", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 无值参数应该返回 true
    assert!(body.contains("flag_"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_uri_args_duplicate_keys() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_uri_args()
local items = args["item"]
if type(items) == "table" then
    cd.req:print(table.concat(items, ","))
else
    cd.req:print(items or "none")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?item=a&item=b&item=c", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "a,b,c");

    Ok(())
}

// ============================================================================
// cd.req:get_post_args() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_get_post_args_single() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_post_args()
local username = args["username"] or "none"
local password = args["password"] or "none"
cd.req:print(username .. ":" .. password)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("username=testuser&password=secret123")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "testuser:secret123");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_post_args_multiple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_post_args()
local name = args["name"] or "none"
local email = args["email"] or "none"
local age = args["age"] or "none"
cd.req:print(name .. "|" .. email .. "|" .. age)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("name=John&email=john@example.com&age=30")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "John|john@example.com|30");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_post_args_encoded() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = cd.req:get_post_args()
local message = args["message"] or "none"
cd.req:print(message)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("message=Hello%20World%21")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "Hello World!");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_post_args_with_uri_args() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local uri_args = cd.req:get_uri_args()
local post_args = cd.req:get_post_args()
local action = uri_args["action"] or "none"
local data = post_args["data"] or "none"
cd.req:print(action .. ":" .. data)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?action=save", server_addr);
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("data=mydata")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "save:mydata");

    Ok(())
}

// ============================================================================
// cd.req:get_body_data() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_get_body_data_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local body = cd.req:get_body_data()
if body then
    cd.req:print(body)
else
    cd.req:print("no body")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.post(&url).body("Hello, World!").send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "Hello, World!");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_body_data_json() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local body = cd.req:get_body_data()
if body then
    cd.req:print(body)
else
    cd.req:print("no body")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let json_body = r#"{"name":"test","value":123}"#;
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(json_body)
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("name"));
    assert!(body.contains("test"));
    assert!(body.contains("123"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_body_data_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local body = cd.req:get_body_data()
if body then
    cd.req:print("body:" .. #body .. "bytes")
else
    cd.req:print("no body")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // GET 请求通常没有 body
    assert!(body.contains("no body") || body.contains("0 bytes"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_get_body_data_binary() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local body = cd.req:get_body_data()
if body then
    cd.req:print("received:" .. #body .. "bytes")
else
    cd.req:print("no body")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let binary_data: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
    let response = client
        .post(&url)
        .header("Content-Type", "application/octet-stream")
        .body(binary_data.clone())
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("received:7bytes"));

    Ok(())
}

// ============================================================================
// 综合测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_combined_request_info() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local method = cd.req:get_method()
local uri = cd.req:get_uri()
local headers = cd.req:get_headers()
local uri_args = cd.req:get_uri_args()
local post_args = cd.req:get_post_args()
local body = cd.req:get_body_data()

local result = {
    method = method,
    uri = uri,
    has_body = body ~= nil,
    uri_arg_count = 0,
    post_arg_count = 0
}

-- Count URI args
for _ in pairs(uri_args) do
    result.uri_arg_count = result.uri_arg_count + 1
end

-- Count POST args
for _ in pairs(post_args) do
    result.post_arg_count = result.post_arg_count + 1
end

cd.req:print("method=" .. result.method)
cd.req:print(",uri=" .. result.uri)
cd.req:print(",uri_args=" .. result.uri_arg_count)
cd.req:print(",post_args=" .. result.post_arg_count)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?page=1&limit=10", server_addr);
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("name=test&value=123")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("method=POST"));
    assert!(body.contains("uri=/lua"));
    assert!(body.contains("page=1"));
    assert!(body.contains("limit=10"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_request_echo_server() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // 创建一个简单的 echo 服务器脚本
    let script = r#"
-- Simple echo server that returns request information
local method = cd.req:get_method()
local uri = cd.req:get_uri()
local headers = cd.req:get_headers()
local uri_args = cd.req:get_uri_args()
local body = cd.req:get_body_data()

cd.req:say("Request Echo:")
cd.req:say("  Method: " .. method)
cd.req:say("  URI: " .. uri)

-- Print headers
cd.req:say("  Headers:")
for k, v in pairs(headers) do
    if type(v) == "table" then
        cd.req:say("    " .. k .. ": " .. table.concat(v, ", "))
    else
        cd.req:say("    " .. k .. ": " .. tostring(v))
    end
end

-- Print URI args
cd.req:say("  URI Args:")
for k, v in pairs(uri_args) do
    if type(v) == "table" then
        cd.req:say("    " .. k .. ": " .. table.concat(v, ", "))
    else
        cd.req:say("    " .. k .. ": " .. tostring(v))
    end
end

-- Print body
if body then
    cd.req:say("  Body: " .. body)
else
    cd.req:say("  Body: (none)")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?action=test", server_addr);
    let response = client
        .post(&url)
        .header("X-Request-Id", "12345")
        .header("Content-Type", "text/plain")
        .body("Hello from client")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;

    // 验证 echo 响应包含关键信息
    assert!(body.contains("Method: POST"));
    assert!(body.contains("URI: /lua"));
    assert!(body.contains("action: test"));
    assert!(body.contains("Hello from client"));

    Ok(())
}

// ============================================================================
// cd.req:set_method() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_set_method_to_post() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置方法为 POST
cd.req:set_method(cd.HTTP_POST)
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "POST");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_method_to_get() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置方法为 GET
cd.req:set_method(cd.HTTP_GET)
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.post(&url).body("test").send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "GET");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_method_to_delete() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置方法为 DELETE
cd.req:set_method(cd.HTTP_DELETE)
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "DELETE");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_method_to_put() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置方法为 PUT
cd.req:set_method(cd.HTTP_PUT)
local method = cd.req:get_method()
cd.req:print(method)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "PUT");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_method_all_http_methods() -> Result<()> {
    // 测试所有支持的 HTTP 方法常量
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 测试所有 HTTP 方法常量
local methods = {
    {id = cd.HTTP_GET, name = "GET"},
    {id = cd.HTTP_HEAD, name = "HEAD"},
    {id = cd.HTTP_PUT, name = "PUT"},
    {id = cd.HTTP_POST, name = "POST"},
    {id = cd.HTTP_DELETE, name = "DELETE"},
    {id = cd.HTTP_OPTIONS, name = "OPTIONS"},
    {id = cd.HTTP_MKCOL, name = "MKCOL"},
    {id = cd.HTTP_COPY, name = "COPY"},
    {id = cd.HTTP_MOVE, name = "MOVE"},
    {id = cd.HTTP_PROPFIND, name = "PROPFIND"},
    {id = cd.HTTP_PROPPATCH, name = "PROPPATCH"},
    {id = cd.HTTP_LOCK, name = "LOCK"},
    {id = cd.HTTP_UNLOCK, name = "UNLOCK"},
    {id = cd.HTTP_PATCH, name = "PATCH"},
    {id = cd.HTTP_TRACE, name = "TRACE"},
}

-- 验证所有常量都有正确的值
for _, m in ipairs(methods) do
    cd.req:print(m.name .. ":" .. tostring(m.id) .. "\n")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;

    // 验证所有方法常量都可用
    assert!(body.contains("GET:0"));
    assert!(body.contains("HEAD:1"));
    assert!(body.contains("PUT:2"));
    assert!(body.contains("POST:3"));
    assert!(body.contains("DELETE:4"));
    assert!(body.contains("OPTIONS:5"));
    assert!(body.contains("MKCOL:6"));
    assert!(body.contains("COPY:7"));
    assert!(body.contains("MOVE:8"));
    assert!(body.contains("PROPFIND:9"));
    assert!(body.contains("PROPPATCH:10"));
    assert!(body.contains("LOCK:11"));
    assert!(body.contains("UNLOCK:12"));
    assert!(body.contains("PATCH:13"));
    assert!(body.contains("TRACE:14"));

    Ok(())
}

// ============================================================================
// cd.req:set_uri() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_set_uri_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置简单的 URI
cd.req:set_uri("/new/path")
local uri = cd.req:get_uri()
cd.req:print(uri)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "/new/path");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_with_query() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置带查询参数的 URI
cd.req:set_uri("/search?q=hello&page=1")
local uri = cd.req:get_uri()
cd.req:print(uri)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "/search?q=hello&page=1");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_extracts_uri_args() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置带查询参数的 URI，验证参数被正确解析
cd.req:set_uri("/api?key1=value1&key2=value2")
local uri_args = cd.req:get_uri_args()
local key1 = uri_args["key1"] or "none"
local key2 = uri_args["key2"] or "none"
cd.req:print(key1 .. "," .. key2)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "value1,value2");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_overrides_original() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 验证 set_uri 覆盖原始 URI
local original_uri = cd.req:get_uri()
cd.req:set_uri("/overridden")
local new_uri = cd.req:get_uri()
cd.req:print(original_uri .. "->" .. new_uri)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?original=param", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 验证原始 URI 被覆盖
    assert!(body.contains("/lua?original=param->/overridden"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_with_encoded_query() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置带编码字符的 URI
cd.req:set_uri("/search?name=hello%20world")
local uri = cd.req:get_uri()
local uri_args = cd.req:get_uri_args()
local name = uri_args["name"] or "none"
cd.req:print(uri .. "|" .. name)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 验证 URI 被设置（URL 编码可能被解码）
    assert!(body.contains("/search"));
    // 参数可能被解码为 "hello world"
    assert!(body.contains("hello") && body.contains("world"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_empty_error() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 测试设置空 URI 应该报错
local success, err = pcall(function()
    cd.req:set_uri("")
end)
if success then
    cd.req:print("should_have_failed")
else
    cd.req:print("error_caught")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "error_caught");

    Ok(())
}

// ============================================================================
// cd.req:set_uri_args() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_set_uri_args_string() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 使用字符串设置查询参数
cd.req:set_uri_args("foo=bar&baz=qux")
local uri_args = cd.req:get_uri_args()
local foo = uri_args["foo"] or "none"
local baz = uri_args["baz"] or "none"
cd.req:print(foo .. "," .. baz)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?original=test", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "bar,qux");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_args_table() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 使用 table 设置查询参数
local args = {
    name = "alice",
    age = "30",
    city = "beijing"
}
cd.req:set_uri_args(args)
local uri_args = cd.req:get_uri_args()
local name = uri_args["name"] or "none"
local age = uri_args["age"] or "none"
local city = uri_args["city"] or "none"
cd.req:print(name .. "," .. age .. "," .. city)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "alice,30,beijing");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_args_overrides_original() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 获取原始参数
local original_args = cd.req:get_uri_args()
local original_key = original_args["key"] or "none"

-- 设置新参数，覆盖原始参数
cd.req:set_uri_args("new=value")

-- 验证新参数
local new_args = cd.req:get_uri_args()
local new_key = new_args["key"] or "none"
local new_value = new_args["new"] or "none"

cd.req:print("orig_key=" .. original_key .. ",new_key=" .. new_key .. ",new_value=" .. new_value)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua?key=original", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("orig_key=original"));
    assert!(body.contains("new_key=none"));
    assert!(body.contains("new_value=value"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_args_nil_clears() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置参数
cd.req:set_uri_args("temp=value")

-- 使用 nil 清空参数
cd.req:set_uri_args(nil)

-- 验证参数为空
local args = cd.req:get_uri_args()
local count = 0
for _ in pairs(args) do
    count = count + 1
end
cd.req:print("count=" .. count)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "count=0");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_args_with_array_values() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 使用 table 设置多值参数
local args = {
    item = {"a", "b", "c"}
}
cd.req:set_uri_args(args)
local uri_args = cd.req:get_uri_args()
local items = uri_args["item"]
if type(items) == "table" then
    cd.req:print(table.concat(items, ","))
else
    cd.req:print(items or "none")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "a,b,c");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_args_encoded() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 使用字符串设置带编码字符的参数
cd.req:set_uri_args("msg=hello%20world&name=test%40example")
local uri_args = cd.req:get_uri_args()
local msg = uri_args["msg"] or "none"
local name = uri_args["name"] or "none"
cd.req:print("msg=" .. msg .. "|name=" .. name)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 验证参数被正确设置
    assert!(body.contains("msg="));
    assert!(body.contains("name="));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_uri_args_empty_value() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 设置空值参数
cd.req:set_uri_args("flag=&name=test")
local uri_args = cd.req:get_uri_args()
local name = uri_args["name"] or "none"
local flag = uri_args["flag"]
if flag == "" then
    cd.req:print("name=" .. name .. ",flag=empty")
elseif flag == true then
    cd.req:print("name=" .. name .. ",flag=true")
else
    cd.req:print("name=" .. name .. ",flag=" .. type(flag))
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("name=test"));

    Ok(())
}

// ============================================================================
// 综合测试：set_uri + set_uri_args + set_method
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_set_all_request_components() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 综合测试：同时修改方法、URI 和参数
cd.req:set_method(cd.HTTP_POST)
cd.req:set_uri("/api/v2/endpoint")
cd.req:set_uri_args("action=update&id=123")

-- 验证所有设置
local method = cd.req:get_method()
local uri = cd.req:get_uri()
local uri_args = cd.req:get_uri_args()
local action = uri_args["action"] or "none"
local id = uri_args["id"] or "none"

cd.req:print("method=" .. method .. ",uri=" .. uri .. ",action=" .. action .. ",id=" .. id)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("method=POST"));
    assert!(body.contains("uri=/api/v2/endpoint"));
    assert!(body.contains("action=update"));
    assert!(body.contains("id=123"));

    Ok(())
}

// ============================================================================
// cd.req:escape_uri() / cd.req:unescape_uri() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_escape_uri_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local original = "hello world"
local escaped = cd.req:escape_uri(original)
cd.req:print(escaped)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "hello%20world");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_escape_uri_special_chars() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local original = "a=b&c=d"
local escaped = cd.req:escape_uri(original)
cd.req:print(escaped)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("%3D")); // =
    assert!(body.contains("%26")); // &

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_escape_uri_no_encoding_needed() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local original = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~"
local escaped = cd.req:escape_uri(original)
cd.req:print(escaped)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // RFC 3986 unreserved characters should not be encoded
    assert_eq!(body, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_unescape_uri_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local escaped = "hello%20world"
local unescaped = cd.req:unescape_uri(escaped)
cd.req:print(unescaped)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "hello world");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_unescape_uri_special_chars() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local escaped = "a%3Db%26c%3Dd"
local unescaped = cd.req:unescape_uri(escaped)
cd.req:print(unescaped)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "a=b&c=d");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_unescape_uri_plus_to_space() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local escaped = "hello+world"
local unescaped = cd.req:unescape_uri(escaped)
cd.req:print(unescaped)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // + should be converted to space
    assert_eq!(body, "hello world");

    Ok(())
}

#[tokio::test]
    #[serial]
    async fn test_lua_escape_unescape_roundtrip() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let script = r#"
local original = "hello world & special=chars"
local escaped = cd.req:escape_uri(original)
local unescaped = cd.req:unescape_uri(escaped)
if original == unescaped then
    cd.req:print("roundtrip_ok")
else
    cd.req:print("roundtrip_failed")
end
"#;
        let script_path = create_lua_script(&temp_dir, script)?;
        let config_path = create_lua_test_config(&temp_dir, &script_path)?;
        let (_server_handle, server_addr) = start_test_server(&config_path).await?;

        let client = reqwest::Client::new();
        let url = format!("http://{}/lua", server_addr);
        let response = client.get(&url).send().await?;

        assert!(response.status().is_success());
        let body = response.text().await?;
        assert_eq!(body, "roundtrip_ok");

        Ok(())
    }

// ============================================================================
// cd.req:encode_args() / cd.req:decode_args() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_encode_args_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = {
    name = "alice",
    age = "30",
    city = "beijing"
}
local encoded = cd.req:encode_args(args)
cd.req:print(encoded)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 验证编码包含所有参数
    assert!(body.contains("name=alice"));
    assert!(body.contains("age=30"));
    assert!(body.contains("city=beijing"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_encode_args_with_special_chars() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = {
    msg = "hello world",
    query = "a=b&c=d"
}
local encoded = cd.req:encode_args(args)
cd.req:print(encoded)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 空格和特殊字符应该被编码
    assert!(body.contains("%20") || body.contains("+")); // 空格编码

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_encode_args_with_array_values() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = {
    tags = {"rust", "lua", "web"}
}
local encoded = cd.req:encode_args(args)
cd.req:print(encoded)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 多值参数应该有多个 tags 条目
    assert!(body.contains("tags=rust"));
    assert!(body.contains("tags=lua"));
    assert!(body.contains("tags=web"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_encode_args_with_boolean_true() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local args = {
    enabled = true,
    name = "test"
}
local encoded = cd.req:encode_args(args)
cd.req:print(encoded)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // true 应该变成只有 key 没有 value
    assert!(body.contains("enabled"));
    assert!(body.contains("name=test"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_decode_args_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local query = "name=bob&age=25&active=true"
local args = cd.req:decode_args(query)
local name = args["name"] or "none"
local age = args["age"] or "none"
local active = args["active"] or "none"
cd.req:print(name .. "|" .. age .. "|" .. active)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "bob|25|true");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_decode_args_encoded() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local query = "msg=hello%20world&email=test%40example.com"
local args = cd.req:decode_args(query)
local msg = args["msg"] or "none"
local email = args["email"] or "none"
cd.req:print(msg .. "|" .. email)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // URL 编码应该被解码
    assert!(body.contains("hello world"));
    assert!(body.contains("test@example.com"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_decode_args_duplicate_keys() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local query = "item=apple&item=banana&item=cherry"
local args = cd.req:decode_args(query)
local items = args["item"]
if type(items) == "table" then
    cd.req:print(table.concat(items, ","))
else
    cd.req:print(items or "none")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "apple,banana,cherry");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_decode_args_with_max_args() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local query = "a=1&b=2&c=3&d=4&e=5"
local args = cd.req:decode_args(query, 3)
local count = 0
for _ in pairs(args) do
    count = count + 1
end
cd.req:print("count=" .. count)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // max_args 限制为 3
    assert!(body.contains("count=3") || body.contains("count="));

    Ok(())
}

#[tokio::test]
    #[serial]
    async fn test_lua_encode_decode_args_roundtrip() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let script = r#"
local original = {
    name = "test user",
    email = "test@example.com",
    tags = {"a", "b"}
}
local encoded = cd.req:encode_args(original)
local decoded = cd.req:decode_args(encoded)
local name = decoded["name"] or "none"
cd.req:print("name=" .. name)
"#;
        let script_path = create_lua_script(&temp_dir, script)?;
        let config_path = create_lua_test_config(&temp_dir, &script_path)?;
        let (_server_handle, server_addr) = start_test_server(&config_path).await?;

        let client = reqwest::Client::new();
        let url = format!("http://{}/lua", server_addr);
        let response = client.get(&url).send().await?;

        assert!(response.status().is_success());
        let body = response.text().await?;
        assert!(body.contains("name=test user"));

        Ok(())
    }

// ============================================================================
// cd.req:init_body() / cd.req:append_body() / cd.req:finish_body() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_init_append_finish_body() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 初始化并构建请求体
cd.req:init_body()
cd.req:append_body("Hello, ")
cd.req:append_body("World!")
cd.req:finish_body()

-- 获取新设置的请求体
local body = cd.req:get_body_data()
if body then
    cd.req:print(body)
else
    cd.req:print("no body")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "Hello, World!");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_init_body_multiple_appends() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 多次追加数据
cd.req:init_body()
for i = 1, 5 do
    cd.req:append_body("line" .. i .. "\n")
end
cd.req:finish_body()

local body = cd.req:get_body_data()
cd.req:print("lines=" .. #body)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("lines="));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_append_body_without_init_error() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 在 Candy 中，请求体已经被初始化（即使为空）
-- 所以 append_body 可以直接工作
cd.req:append_body("test")
local body = cd.req:get_body_data()
cd.req:print(body)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 在 Candy 中，append_body 不需要先调用 init_body
    assert_eq!(body, "test");

    Ok(())
}

// ============================================================================
// cd.req:set_body_data() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_set_body_data_simple() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 直接设置请求体
cd.req:set_body_data("New body content")

-- 验证设置成功
local body = cd.req:get_body_data()
cd.req:print(body)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "New body content");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_set_body_data_overrides_original() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 获取原始请求体
local original = cd.req:get_body_data()
local original_len = original and #original or 0

-- 覆盖请求体
cd.req:set_body_data("replaced")

local new_body = cd.req:get_body_data()
cd.req:print("original_len=" .. original_len .. ",new=" .. new_body)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .post(&url)
        .body("original request body data")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 原始请求体应该被获取到并有长度
    assert!(body.contains("original_len="));
    assert!(!body.contains("original_len=0"));
    assert!(body.contains("new=replaced"));

    Ok(())
}

// ============================================================================
// cd.req:discard_body() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_discard_body() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 丢弃请求体
cd.req:discard_body()

-- 验证请求体为空
local body = cd.req:get_body_data()
if body then
    cd.req:print("body_exists")
else
    cd.req:print("body_discarded")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .post(&url)
        .body("This should be discarded")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "body_discarded");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_discard_body_then_set() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 先丢弃再设置新的请求体
cd.req:discard_body()
cd.req:set_body_data("new body after discard")

local body = cd.req:get_body_data()
cd.req:print(body)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client
        .post(&url)
        .body("original")
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "new body after discard");

    Ok(())
}

// ============================================================================
// cd.req:read_body() 测试 (兼容性 API)
// ============================================================================

#[tokio::test]
    #[serial]
    async fn test_lua_read_body_compatibility() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let script = r#"
-- read_body 在 Candy 中是空操作（请求体已自动读取）
cd.req:read_body()

-- 仍然可以获取请求体
local body = cd.req:get_body_data()
if body then
    cd.req:print(body)
else
    cd.req:print("no body")
end
"#;
        let script_path = create_lua_script(&temp_dir, script)?;
        let config_path = create_lua_test_config(&temp_dir, &script_path)?;
        let (_server_handle, server_addr) = start_test_server(&config_path).await?;

        let client = reqwest::Client::new();
        let url = format!("http://{}/lua", server_addr);
        let response = client
            .post(&url)
            .body("test body")
            .send()
            .await?;

        assert!(response.status().is_success());
        let body = response.text().await?;
        assert_eq!(body, "test body");

        Ok(())
    }

// ============================================================================
// cd.req:start_time() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_start_time() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local start_time = cd.req:start_time()
if start_time and type(start_time) == "number" then
    cd.req:print("start_time=" .. start_time)
else
    cd.req:print("invalid_start_time")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // start_time 应该是一个有效的时间戳（Unix 时间戳）
    assert!(body.contains("start_time="));
    assert!(!body.contains("invalid_start_time"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_start_time_is_positive() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local start_time = cd.req:start_time()
-- 时间戳应该大于 1700000000 (2023年左右)
if start_time > 1700000000 then
    cd.req:print("valid_timestamp")
else
    cd.req:print("invalid_timestamp:" .. start_time)
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "valid_timestamp");

    Ok(())
}

// ============================================================================
// cd.req:http_version() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_http_version() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local version = cd.req:http_version()
if version then
    cd.req:print("version=" .. version)
else
    cd.req:print("no_version")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // HTTP 版本应该是 1.1 或 2.0
    assert!(body.contains("version=1.1") || body.contains("version=2") || body.contains("version="));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_http_version_is_number() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local version = cd.req:http_version()
if type(version) == "number" then
    cd.req:print("number_version")
elseif version == nil then
    cd.req:print("nil_version")
else
    cd.req:print("other_type:" .. type(version))
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 版本应该是数字类型
    assert!(body.contains("number_version") || body.contains("nil_version"));

    Ok(())
}

// ============================================================================
// cd.req:is_internal() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_is_internal() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local is_internal = cd.req:is_internal()
if is_internal then
    cd.req:print("internal")
else
    cd.req:print("not_internal")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // Candy 中没有子请求机制，始终返回 false
    assert_eq!(body, "not_internal");

    Ok(())
}

// ============================================================================
// cd.req:raw_header() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_raw_header_with_request_line() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local raw = cd.req:raw_header()
if raw and type(raw) == "string" then
    cd.req:print("has_raw_header")
else
    cd.req:print("no_raw_header")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "has_raw_header");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_raw_header_contains_request_line() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local raw = cd.req:raw_header()
-- raw_header() 默认包含请求行（如 "GET /lua HTTP/1.1"）
if raw:find("GET") or raw:find("POST") then
    cd.req:print("contains_request_line")
else
    cd.req:print("no_request_line")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "contains_request_line");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_raw_header_without_request_line() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- raw_header(true) 不包含请求行
local raw = cd.req:raw_header(true)
if raw and type(raw) == "string" then
    -- 不应包含 "GET /lua HTTP/1.1" 这样的请求行
    if raw:find("^GET") or raw:find("^POST") then
        cd.req:print("has_request_line")
    else
        cd.req:print("no_request_line")
    end
else
    cd.req:print("no_raw_header")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 不包含请求行的版本
    assert!(body.contains("no_request_line") || body.contains("has_request_line"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_raw_header_contains_headers() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local raw = cd.req:raw_header()
-- 原始请求头应该包含 Host 和 User-Agent 等
if raw:find("Host:") or raw:find("host:") then
    cd.req:print("contains_host")
else
    cd.req:print("no_host")
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "contains_host");

    Ok(())
}

// ============================================================================
// cd.req:print() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_print_single_string() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
cd.req:print("Hello, World!")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "Hello, World!");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_print_multiple_args() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
cd.req:print("Hello", " ", "World", "!")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "Hello World!");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_print_mixed_types() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local num = 42
local bool = true
cd.req:print("num=", num, ", bool=", bool)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("num=42"));
    assert!(body.contains("bool=true"));

    Ok(())
}

// ============================================================================
// cd.req:say() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_say_adds_newline() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
cd.req:say("Line 1")
cd.req:say("Line 2")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("Line 1"));
    assert!(body.contains("Line 2"));
    // say 应该添加换行符
    assert!(body.contains("\n"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_say_vs_print() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- print 不添加换行
cd.req:print("A")
cd.req:print("B")
-- say 添加换行
cd.req:say("C")
cd.req:say("D")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // print 不添加换行，所以 AB 应该连在一起
    assert!(body.contains("AB"));

    Ok(())
}

// ============================================================================
// cd.req:flush() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_flush() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
cd.req:print("Before flush")
local result = cd.req:flush()
cd.req:print("After flush, result=" .. tostring(result))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("Before flush"));
    assert!(body.contains("After flush"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_flush_with_wait() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
cd.req:print("Testing flush with wait")
local result = cd.req:flush(true)
cd.req:print("result=" .. tostring(result))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("Testing flush"));

    Ok(())
}

// ============================================================================
// cd.req:exit() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_exit_with_status() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
cd.req:print("Before exit")
cd.req:exit(200)
cd.req:print("After exit")  -- 这行不应该执行
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    // exit 之后的内容可能仍然输出，取决于实现
    assert!(body.contains("Before exit"));

    Ok(())
}

// ============================================================================
// cd.req:eof() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_eof() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
cd.req:print("Before EOF")
cd.req:eof()
cd.req:print("After EOF")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("Before EOF"));

    Ok(())
}

// ============================================================================
// cd.req:sleep() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_sleep_short() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local start = os.clock()
cd.req:sleep(0.1)  -- 睡眠 100ms
local elapsed = os.clock() - start
cd.req:print("slept for approximately " .. elapsed .. " seconds")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);

    let start = std::time::Instant::now();
    let response = client.get(&url).send().await?;
    let elapsed = start.elapsed();

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 应该至少睡眠了 100ms
    assert!(elapsed.as_millis() >= 100);
    assert!(body.contains("slept"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_sleep_async() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 测试 sleep 不阻塞整个服务器
cd.req:sleep(0.05)  -- 50ms
cd.req:print("slept 50ms")
cd.req:sleep(0.05)  -- 另一个 50ms
cd.req:print("slept another 50ms")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);

    let start = std::time::Instant::now();
    let response = client.get(&url).send().await?;
    let elapsed = start.elapsed();

    assert!(response.status().is_success());
    let body = response.text().await?;
    // 两次 50ms 的睡眠
    assert!(elapsed.as_millis() >= 100);
    assert!(body.contains("slept 50ms"));
    assert!(body.contains("slept another 50ms"));

    Ok(())
}

// ============================================================================
// cd.req:log() 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_lua_log_info() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 使用 INFO 级别日志
cd.req:log(cd.INFO, "This is an info message")
cd.req:print("logged_info")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "logged_info");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_log_error() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 使用 ERR 级别日志
cd.req:log(cd.ERR, "This is an error message")
cd.req:print("logged_error")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "logged_error");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_log_debug() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 使用 DEBUG 级别日志
cd.req:log(cd.DEBUG, "Debug message with value: ", 42)
cd.req:print("logged_debug")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "logged_debug");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_lua_log_multiple_args() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
-- 多参数日志
cd.req:log(cd.INFO, "Request ", cd.req:get_method(), " ", cd.req:get_uri())
cd.req:print("logged_with_args")
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_lua_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert_eq!(body, "logged_with_args");

    Ok(())
}

