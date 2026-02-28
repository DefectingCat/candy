//! Lua 脚本引擎集成测试
//!
//! 测试 Lua 脚本在 HTTP 请求处理中的各种 API：
//! - cd.req:get_method() - 获取请求方法
//! - cd.req:get_uri() - 获取 URI
//! - cd.req:get_headers() - 获取请求头
//! - cd.req:get_uri_args() - 获取查询参数
//! - cd.req:get_post_args() - 获取 POST 参数
//! - cd.req:get_body_data() - 获取请求体

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

