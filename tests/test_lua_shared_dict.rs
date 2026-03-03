//! Lua 共享字典集成测试
//!
//! 测试 ngx.shared.DICT 的所有 API 方法：
//! - get/get_stale
//! - set/safe_set
//! - add/safe_add
//! - replace/delete
//! - incr
//! - flush_all/flush_expired/get_keys
//! - lpush/rpush/lpop/rpop/llen

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

/// 创建包含共享字典和 Lua 脚本路由的测试配置
fn create_shared_dict_test_config(temp_dir: &TempDir, lua_script_path: &PathBuf) -> Result<PathBuf> {
    let config_path = temp_dir.path().join("config.toml");

    let config_content = format!(
        r#"
[log]
level = "debug"

# 定义共享字典
[[lua_shared_dict]]
name = "cache"
size = "1m"

[[lua_shared_dict]]
name = "counter"
size = "100k"

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
// get/set 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_set_get() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
local ok, err, forcible = dict:set("key1", "value1")
if not ok then
    cd.req:print("set failed: " .. tostring(err))
else
    local val, flags = dict:get("key1")
    cd.req:print(val)
end
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "value1");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_set_with_flags() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1", 0, 42)
local val, flags = dict:get("key1")
cd.req:print(val .. ":" .. tostring(flags))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "value1:42");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_set_with_exptime() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
-- 设置 0.1 秒过期
dict:set("key1", "value1", 0.1)
local val1, _ = dict:get("key1")
cd.req:print(tostring(val1) .. ",")
-- 等待 0.2 秒
cd.req:sleep(0.2)
local val2, _ = dict:get("key1")
cd.req:print(tostring(val2))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "value1,nil");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_get_nonexistent() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
local val, flags = dict:get("nonexistent_key")
cd.req:print(tostring(val) .. "," .. tostring(flags))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "nil,nil");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_get_stale() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
-- 设置 0.1 秒过期
dict:set("key1", "value1", 0.1)
-- 等待过期
cd.req:sleep(0.2)
-- get_stale 应该能获取到过期值
local val, flags, stale = dict:get_stale("key1")
cd.req:print(tostring(val) .. "," .. tostring(stale))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "value1,true");

    Ok(())
}

// ============================================================================
// add 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_add_success() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
local ok, err, forcible = dict:add("key1", "value1")
cd.req:print(tostring(ok) .. "," .. tostring(err))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "true,nil");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_add_exists() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1")
local ok, err, forcible = dict:add("key1", "value2")
cd.req:print(tostring(ok) .. "," .. tostring(err))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "false,exists");

    Ok(())
}

// ============================================================================
// safe_add 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_safe_add_success() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
local ok, err = dict:safe_add("key1", "value1")
cd.req:print(tostring(ok) .. "," .. tostring(err))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "true,nil");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_safe_add_exists() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1")
local ok, err = dict:safe_add("key1", "value2")
cd.req:print(tostring(ok) .. "," .. tostring(err))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    // safe_add 对于已存在的键返回 false
    assert_eq!(body, "false,exists");

    Ok(())
}

// ============================================================================
// replace 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_replace_success() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1")
local ok, err, forcible = dict:replace("key1", "value2")
local val, _ = dict:get("key1")
cd.req:print(tostring(ok) .. "," .. tostring(val))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "true,value2");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_replace_not_found() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
local ok, err, forcible = dict:replace("nonexistent", "value1")
cd.req:print(tostring(ok) .. "," .. tostring(err))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "false,not found");

    Ok(())
}

// ============================================================================
// delete 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_delete() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1")
local result = dict:delete("key1")
local val, _ = dict:get("key1")
cd.req:print(tostring(result) .. "," .. tostring(val))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "true,nil");

    Ok(())
}

// ============================================================================
// incr 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_incr_basic() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.counter
dict:set("count", "10")
local newval, err, forcible = dict:incr("count", 5)
cd.req:print(tostring(newval))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "15");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_incr_with_init() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.counter
-- 不存在的键，使用初始值
local newval, err, forcible = dict:incr("newcount", 10, 0)
cd.req:print(tostring(newval))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "10");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_incr_negative() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.counter
dict:set("count", "10")
local newval, err, forcible = dict:incr("count", -3)
cd.req:print(tostring(newval))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "7");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_incr_not_found_no_init() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.counter
-- 不存在的键，无初始值
local newval, err, forcible = dict:incr("nonexistent", 5)
cd.req:print(tostring(newval) .. "," .. tostring(err))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "nil,not found");

    Ok(())
}

// ============================================================================
// 列表操作测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_lpush_rpush() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
local len1, err = dict:lpush("mylist", "item1")
local len2, err = dict:rpush("mylist", "item2")
local len3, err = dict:lpush("mylist", "item0")
cd.req:print(tostring(len1) .. "," .. tostring(len2) .. "," .. tostring(len3))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "1,2,3");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_lpop_rpop() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
-- 创建列表 [a, b, c]
dict:rpush("mylist", "a")
dict:rpush("mylist", "b")
dict:rpush("mylist", "c")

-- lpop 从头部弹出
local val1, err = dict:lpop("mylist")
-- rpop 从尾部弹出
local val2, err = dict:rpop("mylist")

cd.req:print(tostring(val1) .. "," .. tostring(val2))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "a,c");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_llen() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:rpush("mylist", "a")
dict:rpush("mylist", "b")
dict:rpush("mylist", "c")
local len, err = dict:llen("mylist")
cd.req:print(tostring(len))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "3");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_llen_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
-- 不存在的列表，长度为 0
local len, err = dict:llen("nonexistent")
cd.req:print(tostring(len))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "0");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_list_value_not_a_list() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
-- 设置标量值
dict:set("key", "scalar_value")
-- 尝试 lpush 到标量
local len, err = dict:lpush("key", "item")
cd.req:print(tostring(len) .. "," .. tostring(err))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "nil,value not a list");

    Ok(())
}

// ============================================================================
// flush_all/flush_expired/get_keys 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_flush_all() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1")
dict:set("key2", "value2")
dict:flush_all()
local val1, _ = dict:get("key1")
local val2, _ = dict:get("key2")
cd.req:print(tostring(val1) .. "," .. tostring(val2))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "nil,nil");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_flush_expired() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1", 0.1)  -- 0.1 秒过期
dict:set("key2", "value2", 10)   -- 10 秒过期
dict:set("key3", "value3")       -- 永不过期

-- 等待过期
cd.req:sleep(0.2)

-- 清除过期条目
local count = dict:flush_expired()
cd.req:print(tostring(count))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "1");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_get_keys() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1")
dict:set("key2", "value2")
dict:set("key3", "value3")

local keys = dict:get_keys()
-- 按字母顺序排序以便测试
table.sort(keys)
local result = table.concat(keys, ",")
cd.req:print(result)
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "key1,key2,key3");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_shared_dict_get_keys_with_limit() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
dict:set("key1", "value1")
dict:set("key2", "value2")
dict:set("key3", "value3")

local keys = dict:get_keys(2)
cd.req:print(tostring(#keys))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "2");

    Ok(())
}

// ============================================================================
// safe_set 测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_safe_set_success() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local dict = ngx.shared.cache
local ok, err = dict:safe_set("key1", "value1")
local val, _ = dict:get("key1")
cd.req:print(tostring(ok) .. "," .. tostring(val))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "true,value1");

    Ok(())
}

// ============================================================================
// 多请求共享测试（验证跨请求共享）
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_cross_request_sharing() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // 第一个脚本：设置值
    let script1 = r#"
local dict = ngx.shared.cache
dict:set("shared_key", "shared_value")
cd.req:print("set")
"#;
    let script_path1 = create_lua_script(&temp_dir, script1)?;

    // 第二个脚本：读取值
    let script2 = r#"
local dict = ngx.shared.cache
local val, _ = dict:get("shared_key")
cd.req:print(tostring(val))
"#;

    // 创建两个脚本文件
    let script_path2 = temp_dir.path().join("test2.lua");
    fs::write(&script_path2, script2)?;

    // 创建第一个配置
    let config_path = temp_dir.path().join("config.toml");
    let config_content = format!(
        r#"
[log]
level = "debug"

[[lua_shared_dict]]
name = "cache"
size = "1m"

[[host]]
ip = "127.0.0.1"
port = 0
ssl = false
timeout = 75

[[host.route]]
location = "/set"
lua_script = "{}"
lua_code_cache = false

[[host.route]]
location = "/get"
lua_script = "{}"
lua_code_cache = false
"#,
        script_path1.to_str().unwrap(),
        script_path2.to_str().unwrap()
    );
    fs::write(&config_path, config_content)?;

    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();

    // 第一个请求设置值
    let url1 = format!("http://{}/set", server_addr);
    let response1 = client.get(&url1).send().await?;
    assert!(response1.status().is_success());
    assert_eq!(response1.text().await?, "set");

    // 第二个请求读取值
    let url2 = format!("http://{}/get", server_addr);
    let response2 = client.get(&url2).send().await?;
    assert!(response2.status().is_success());
    let body2 = response2.text().await?;
    assert_eq!(body2, "shared_value");

    Ok(())
}

// ============================================================================
// 多字典测试
// ============================================================================

#[tokio::test]
#[serial]
async fn test_shared_dict_multiple_dicts() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let script = r#"
local cache = ngx.shared.cache
local counter = ngx.shared.counter

cache:set("key1", "cache_value")
counter:set("key1", "counter_value")

local val1, _ = cache:get("key1")
local val2, _ = counter:get("key1")

cd.req:print(tostring(val1) .. "," .. tostring(val2))
"#;
    let script_path = create_lua_script(&temp_dir, script)?;
    let config_path = create_shared_dict_test_config(&temp_dir, &script_path)?;
    let (_server_handle, server_addr) = start_test_server(&config_path).await?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/lua", server_addr);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        panic!("Request failed with status {}: {}", status, body);
    }
    let body = response.text().await?;
    assert_eq!(body, "cache_value,counter_value");

    Ok(())
}