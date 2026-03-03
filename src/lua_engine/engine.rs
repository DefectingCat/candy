//! Lua 引擎核心实现
//!
//! 提供 Lua 虚拟机初始化、代码缓存和共享字典管理

use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use mlua::{Function, Lua};
use tracing::{debug, info};

use crate::config::Settings;
use crate::consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};

use super::shared_dict::SharedDict;

// ============================================================================
// HTTP 方法常量
// ============================================================================
pub const HTTP_GET: u16 = 0;
pub const HTTP_HEAD: u16 = 1;
pub const HTTP_PUT: u16 = 2;
pub const HTTP_POST: u16 = 3;
pub const HTTP_DELETE: u16 = 4;
pub const HTTP_OPTIONS: u16 = 5;
pub const HTTP_MKCOL: u16 = 6;
pub const HTTP_COPY: u16 = 7;
pub const HTTP_MOVE: u16 = 8;
pub const HTTP_PROPFIND: u16 = 9;
pub const HTTP_PROPPATCH: u16 = 10;
pub const HTTP_LOCK: u16 = 11;
pub const HTTP_UNLOCK: u16 = 12;
pub const HTTP_PATCH: u16 = 13;
pub const HTTP_TRACE: u16 = 14;

// ============================================================================
// HTTP 状态码常量
// ============================================================================

// 1xx Informational
pub const HTTP_CONTINUE: u16 = 100;
pub const HTTP_SWITCHING_PROTOCOLS: u16 = 101;

// 2xx Success
pub const HTTP_OK: u16 = 200;
pub const HTTP_CREATED: u16 = 201;
pub const HTTP_ACCEPTED: u16 = 202;
pub const HTTP_NO_CONTENT: u16 = 204;
pub const HTTP_PARTIAL_CONTENT: u16 = 206;

// 3xx Redirection
pub const HTTP_SPECIAL_RESPONSE: u16 = 300;
pub const HTTP_MOVED_PERMANENTLY: u16 = 301;
pub const HTTP_MOVED_TEMPORARILY: u16 = 302;
pub const HTTP_SEE_OTHER: u16 = 303;
pub const HTTP_NOT_MODIFIED: u16 = 304;
pub const HTTP_TEMPORARY_REDIRECT: u16 = 307;

// 4xx Client Error
pub const HTTP_BAD_REQUEST: u16 = 400;
pub const HTTP_UNAUTHORIZED: u16 = 401;
pub const HTTP_PAYMENT_REQUIRED: u16 = 402;
pub const HTTP_FORBIDDEN: u16 = 403;
pub const HTTP_NOT_FOUND: u16 = 404;
pub const HTTP_NOT_ALLOWED: u16 = 405;
pub const HTTP_NOT_ACCEPTABLE: u16 = 406;
pub const HTTP_REQUEST_TIMEOUT: u16 = 408;
pub const HTTP_CONFLICT: u16 = 409;
pub const HTTP_GONE: u16 = 410;
pub const HTTP_UPGRADE_REQUIRED: u16 = 426;
pub const HTTP_TOO_MANY_REQUESTS: u16 = 429;
pub const HTTP_CLOSE: u16 = 444;
pub const HTTP_ILLEGAL: u16 = 451;

// 5xx Server Error
pub const HTTP_INTERNAL_SERVER_ERROR: u16 = 500;
pub const HTTP_METHOD_NOT_IMPLEMENTED: u16 = 501;
pub const HTTP_BAD_GATEWAY: u16 = 502;
pub const HTTP_SERVICE_UNAVAILABLE: u16 = 503;
pub const HTTP_GATEWAY_TIMEOUT: u16 = 504;
pub const HTTP_VERSION_NOT_SUPPORTED: u16 = 505;
pub const HTTP_INSUFFICIENT_STORAGE: u16 = 507;

// ============================================================================
// 日志级别常量
// ============================================================================
pub const LOG_EMERG: u8 = 2;
pub const LOG_ALERT: u8 = 4;
pub const LOG_CRIT: u8 = 8;
pub const LOG_ERR: u8 = 16;
pub const LOG_WARN: u8 = 32;
pub const LOG_NOTICE: u8 = 64;
pub const LOG_INFO: u8 = 128;
pub const LOG_DEBUG: u8 = 255;

/// Lua 代码缓存条目
pub struct LuaCodeCacheEntry {
    /// 编译后的 Lua 函数
    pub compiled_func: Function,
    /// 脚本内容的校验和，用于检测脚本是否发生变化
    pub checksum: u64,
}

/// Lua 引擎实例，包含 Lua 虚拟机、代码缓存和共享字典
pub struct LuaEngine {
    /// Lua 虚拟机实例
    pub lua: Lua,
    /// Lua 代码缓存，用于存储编译后的 Lua 脚本
    /// 键：脚本文件路径
    /// 值：(编译后的函数, 脚本内容的校验和)
    pub code_cache: Arc<DashMap<String, LuaCodeCacheEntry>>,
    /// 共享字典存储
    /// 键：字典名称
    /// 值：SharedDict 实例
    pub shared_dicts: Arc<DashMap<String, SharedDict>>,
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LuaEngine {
    /// 创建新的 Lua 引擎实例
    ///
    /// 初始化 Lua 虚拟机并注册 `candy` 全局模块，提供以下功能：
    /// - 日志功能 (log)
    /// - 版本信息访问 (version/name/os/arch/compiler/commit)
    pub fn new() -> Self {
        let lua = Lua::new();
        let code_cache = Arc::new(DashMap::new());
        let shared_dicts = Arc::new(DashMap::new());

        // 创建主模块
        let module = lua.create_table().expect("Failed to create Lua module");

        // 注册日志函数
        Self::register_log_function(&lua, &module);

        // 注册版本信息常量
        Self::register_version_info(&module);

        // 注册 HTTP 常量
        Self::register_http_constants(&module);

        // 注册 cd.shared 模块
        Self::register_shared_dicts(&lua, &shared_dicts);

        // 将 `candy` 模块设置为全局变量
        lua.globals()
            .set("candy", module)
            .expect("设置全局变量 candy 失败");

        Self {
            lua,
            code_cache,
            shared_dicts,
        }
    }

    /// 从配置创建 Lua 引擎实例
    ///
    /// # 参数
    /// - `settings` - 服务器配置
    pub fn from_settings(settings: &Settings) -> Self {
        let lua = Lua::new();
        let code_cache = Arc::new(DashMap::new());
        let shared_dicts = Arc::new(DashMap::new());

        // 初始化共享字典
        if let Some(dicts) = &settings.lua_shared_dict {
            for dict_config in dicts {
                if let Ok(capacity) = dict_config.parse_size() {
                    let dict = SharedDict::new(dict_config.name.clone(), capacity);
                    shared_dicts.insert(dict_config.name.clone(), dict);
                    info!(
                        "Created shared dict '{}' with capacity {} bytes",
                        dict_config.name, capacity
                    );
                }
            }
        }

        // 创建主模块
        let module = lua.create_table().expect("Failed to create Lua module");

        // 注册日志函数
        Self::register_log_function(&lua, &module);

        // 注册版本信息常量
        Self::register_version_info(&module);

        // 注册 HTTP 常量
        Self::register_http_constants(&module);

        // 注册 cd.shared 模块
        Self::register_shared_dicts(&lua, &shared_dicts);

        // 将 `candy` 模块设置为全局变量
        lua.globals()
            .set("candy", module)
            .expect("设置全局变量 candy 失败");

        Self {
            lua,
            code_cache,
            shared_dicts,
        }
    }

    /// 初始化共享字典（动态添加）
    ///
    /// # 参数
    /// - `name` - 字典名称
    /// - `capacity` - 容量（字节）
    pub fn init_shared_dict(&self, name: &str, capacity: usize) {
        if !self.shared_dicts.contains_key(name) {
            let dict = SharedDict::new(name.to_string(), capacity);
            self.shared_dicts.insert(name.to_string(), dict);

            // 注册到 Lua
            let cd_shared: mlua::Table = self
                .lua
                .globals()
                .get::<mlua::Table>("cd")
                .and_then(|cd: mlua::Table| cd.get::<mlua::Table>("shared"))
                .unwrap_or_else(|_| self.lua.create_table().unwrap());

            if let Some(dict) = self.shared_dicts.get(name) {
                let _ = cd_shared.set(name, dict.clone());
            }

            let cd: mlua::Table = self
                .lua
                .globals()
                .get("cd")
                .unwrap_or_else(|_| self.lua.create_table().unwrap());
            let _ = cd.set("shared", cd_shared);
            let _ = self.lua.globals().set("cd", cd);

            info!(
                "Initialized shared dict '{}' with capacity {} bytes",
                name, capacity
            );
        }
    }

    /// 重置共享字典（清空数据，用于测试隔离）
    ///
    /// # 参数
    /// - `name` - 字典名称
    /// - `capacity` - 容量（字节）
    pub fn reset_shared_dict(&self, name: &str, capacity: usize) {
        // 先删除旧的（如果存在）
        self.shared_dicts.remove(name);

        // 创建新的字典
        let dict = SharedDict::new(name.to_string(), capacity);
        self.shared_dicts.insert(name.to_string(), dict);

        // 更新 __candy_shared__ 全局变量
        let shared: mlua::Table = self
            .lua
            .globals()
            .get("__candy_shared__")
            .unwrap_or_else(|_| self.lua.create_table().unwrap());

        if let Some(dict) = self.shared_dicts.get(name) {
            let _ = shared.set(name, dict.clone());
        }

        let _ = self.lua.globals().set("__candy_shared__", shared);
    }

    /// 清理所有共享字典（用于测试隔离）
    pub fn clear_shared_dicts(&self) {
        self.shared_dicts.clear();
    }

    /// 注册共享字典到 Lua 全局变量
    fn register_shared_dicts(lua: &Lua, shared_dicts: &Arc<DashMap<String, SharedDict>>) {
        // 创建 cd 表
        let cd = lua.create_table().expect("Failed to create cd table");

        // 创建 cd.shared 表
        let shared = lua
            .create_table()
            .expect("Failed to create cd.shared table");

        // 注册所有共享字典
        for entry in shared_dicts.iter() {
            let name = entry.key();
            let dict = entry.value();
            shared
                .set(name.clone(), dict.clone())
                .expect("Failed to set shared dict");
        }

        // 设置 cd.shared
        cd.set("shared", shared.clone()).expect("Failed to set cd.shared");

        // 设置全局变量 cd
        lua.globals()
            .set("cd", cd)
            .expect("Failed to set cd global");

        // 同时将 shared 设置为 __candy_shared__ 全局变量
        // 这样在请求处理时可以通过 RequestContext 的 __index 元方法访问
        lua.globals()
            .set("__candy_shared__", shared)
            .expect("Failed to set __candy_shared__ global");
    }

    /// 清除所有 Lua 代码缓存
    #[allow(dead_code)]
    pub fn clear_cache(&self) {
        self.code_cache.clear();
        info!("Lua code cache cleared");
    }

    /// 清除特定脚本的缓存条目
    #[allow(dead_code)]
    pub fn remove_from_cache(&self, script_path: &str) -> bool {
        if self.code_cache.remove(script_path).is_some() {
            info!("Removed Lua script from cache: {}", script_path);
            true
        } else {
            debug!("Lua script not found in cache: {}", script_path);
            false
        }
    }

    /// 获取缓存统计信息
    #[allow(dead_code)]
    pub fn cache_stats(&self) -> (usize, usize, usize) {
        let entry_count = self.code_cache.len();
        let total_memory_estimate =
            self.code_cache.len() * std::mem::size_of::<LuaCodeCacheEntry>();

        // 估算平均条目大小（包含编译后的函数）
        let estimated_bytes = entry_count * 1024; // 假设平均每个编译后的函数是 1KB

        (entry_count, estimated_bytes, total_memory_estimate)
    }

    /// 打印缓存统计信息（用于调试）
    #[allow(dead_code)]
    pub fn print_cache_stats(&self) {
        let (count, estimated_bytes, memory) = self.cache_stats();
        info!(
            "Lua code cache stats: {} entries, ~{} bytes ({} KB), memory: {} bytes",
            count,
            estimated_bytes,
            estimated_bytes / 1024,
            memory
        );
    }

    /// 注册日志函数到主模块
    fn register_log_function(lua: &Lua, module: &mlua::Table) {
        let log_func = lua
            .create_function(move |_, msg: String| {
                info!("Lua: {}", msg);
                Ok(())
            })
            .expect("Failed to create log function");
        module
            .set("log", log_func)
            .expect("Failed to set log method");
    }

    /// 注册版本信息常量到主模块
    fn register_version_info(module: &mlua::Table) {
        module
            .set("version", VERSION)
            .expect("Failed to set version");
        module
            .set("name", NAME)
            .expect("Failed to set application name");
        module.set("os", OS).expect("Failed to set OS info");
        module
            .set("arch", ARCH)
            .expect("Failed to set architecture info");
        module
            .set("compiler", COMPILER)
            .expect("Failed to set compiler info");
        module
            .set("commit", COMMIT)
            .expect("Failed to set commit hash");
    }

    /// 注册 HTTP 常量到主模块
    fn register_http_constants(module: &mlua::Table) {
        // HTTP 方法常量
        module
            .set("HTTP_GET", HTTP_GET)
            .expect("Failed to set HTTP_GET");
        module
            .set("HTTP_HEAD", HTTP_HEAD)
            .expect("Failed to set HTTP_HEAD");
        module
            .set("HTTP_PUT", HTTP_PUT)
            .expect("Failed to set HTTP_PUT");
        module
            .set("HTTP_POST", HTTP_POST)
            .expect("Failed to set HTTP_POST");
        module
            .set("HTTP_DELETE", HTTP_DELETE)
            .expect("Failed to set HTTP_DELETE");
        module
            .set("HTTP_OPTIONS", HTTP_OPTIONS)
            .expect("Failed to set HTTP_OPTIONS");
        module
            .set("HTTP_MKCOL", HTTP_MKCOL)
            .expect("Failed to set HTTP_MKCOL");
        module
            .set("HTTP_COPY", HTTP_COPY)
            .expect("Failed to set HTTP_COPY");
        module
            .set("HTTP_MOVE", HTTP_MOVE)
            .expect("Failed to set HTTP_MOVE");
        module
            .set("HTTP_PROPFIND", HTTP_PROPFIND)
            .expect("Failed to set HTTP_PROPFIND");
        module
            .set("HTTP_PROPPATCH", HTTP_PROPPATCH)
            .expect("Failed to set HTTP_PROPPATCH");
        module
            .set("HTTP_LOCK", HTTP_LOCK)
            .expect("Failed to set HTTP_LOCK");
        module
            .set("HTTP_UNLOCK", HTTP_UNLOCK)
            .expect("Failed to set HTTP_UNLOCK");
        module
            .set("HTTP_PATCH", HTTP_PATCH)
            .expect("Failed to set HTTP_PATCH");
        module
            .set("HTTP_TRACE", HTTP_TRACE)
            .expect("Failed to set HTTP_TRACE");

        // HTTP 状态码常量 - 1xx
        module
            .set("HTTP_CONTINUE", HTTP_CONTINUE)
            .expect("Failed to set HTTP_CONTINUE");
        module
            .set("HTTP_SWITCHING_PROTOCOLS", HTTP_SWITCHING_PROTOCOLS)
            .expect("Failed to set HTTP_SWITCHING_PROTOCOLS");

        // HTTP 状态码常量 - 2xx
        module
            .set("HTTP_OK", HTTP_OK)
            .expect("Failed to set HTTP_OK");
        module
            .set("HTTP_CREATED", HTTP_CREATED)
            .expect("Failed to set HTTP_CREATED");
        module
            .set("HTTP_ACCEPTED", HTTP_ACCEPTED)
            .expect("Failed to set HTTP_ACCEPTED");
        module
            .set("HTTP_NO_CONTENT", HTTP_NO_CONTENT)
            .expect("Failed to set HTTP_NO_CONTENT");
        module
            .set("HTTP_PARTIAL_CONTENT", HTTP_PARTIAL_CONTENT)
            .expect("Failed to set HTTP_PARTIAL_CONTENT");

        // HTTP 状态码常量 - 3xx
        module
            .set("HTTP_SPECIAL_RESPONSE", HTTP_SPECIAL_RESPONSE)
            .expect("Failed to set HTTP_SPECIAL_RESPONSE");
        module
            .set("HTTP_MOVED_PERMANENTLY", HTTP_MOVED_PERMANENTLY)
            .expect("Failed to set HTTP_MOVED_PERMANENTLY");
        module
            .set("HTTP_MOVED_TEMPORARILY", HTTP_MOVED_TEMPORARILY)
            .expect("Failed to set HTTP_MOVED_TEMPORARILY");
        module
            .set("HTTP_SEE_OTHER", HTTP_SEE_OTHER)
            .expect("Failed to set HTTP_SEE_OTHER");
        module
            .set("HTTP_NOT_MODIFIED", HTTP_NOT_MODIFIED)
            .expect("Failed to set HTTP_NOT_MODIFIED");
        module
            .set("HTTP_TEMPORARY_REDIRECT", HTTP_TEMPORARY_REDIRECT)
            .expect("Failed to set HTTP_TEMPORARY_REDIRECT");

        // HTTP 状态码常量 - 4xx
        module
            .set("HTTP_BAD_REQUEST", HTTP_BAD_REQUEST)
            .expect("Failed to set HTTP_BAD_REQUEST");
        module
            .set("HTTP_UNAUTHORIZED", HTTP_UNAUTHORIZED)
            .expect("Failed to set HTTP_UNAUTHORIZED");
        module
            .set("HTTP_PAYMENT_REQUIRED", HTTP_PAYMENT_REQUIRED)
            .expect("Failed to set HTTP_PAYMENT_REQUIRED");
        module
            .set("HTTP_FORBIDDEN", HTTP_FORBIDDEN)
            .expect("Failed to set HTTP_FORBIDDEN");
        module
            .set("HTTP_NOT_FOUND", HTTP_NOT_FOUND)
            .expect("Failed to set HTTP_NOT_FOUND");
        module
            .set("HTTP_NOT_ALLOWED", HTTP_NOT_ALLOWED)
            .expect("Failed to set HTTP_NOT_ALLOWED");
        module
            .set("HTTP_NOT_ACCEPTABLE", HTTP_NOT_ACCEPTABLE)
            .expect("Failed to set HTTP_NOT_ACCEPTABLE");
        module
            .set("HTTP_REQUEST_TIMEOUT", HTTP_REQUEST_TIMEOUT)
            .expect("Failed to set HTTP_REQUEST_TIMEOUT");
        module
            .set("HTTP_CONFLICT", HTTP_CONFLICT)
            .expect("Failed to set HTTP_CONFLICT");
        module
            .set("HTTP_GONE", HTTP_GONE)
            .expect("Failed to set HTTP_GONE");
        module
            .set("HTTP_UPGRADE_REQUIRED", HTTP_UPGRADE_REQUIRED)
            .expect("Failed to set HTTP_UPGRADE_REQUIRED");
        module
            .set("HTTP_TOO_MANY_REQUESTS", HTTP_TOO_MANY_REQUESTS)
            .expect("Failed to set HTTP_TOO_MANY_REQUESTS");
        module
            .set("HTTP_CLOSE", HTTP_CLOSE)
            .expect("Failed to set HTTP_CLOSE");
        module
            .set("HTTP_ILLEGAL", HTTP_ILLEGAL)
            .expect("Failed to set HTTP_ILLEGAL");

        // HTTP 状态码常量 - 5xx
        module
            .set("HTTP_INTERNAL_SERVER_ERROR", HTTP_INTERNAL_SERVER_ERROR)
            .expect("Failed to set HTTP_INTERNAL_SERVER_ERROR");
        module
            .set("HTTP_METHOD_NOT_IMPLEMENTED", HTTP_METHOD_NOT_IMPLEMENTED)
            .expect("Failed to set HTTP_METHOD_NOT_IMPLEMENTED");
        module
            .set("HTTP_BAD_GATEWAY", HTTP_BAD_GATEWAY)
            .expect("Failed to set HTTP_BAD_GATEWAY");
        module
            .set("HTTP_SERVICE_UNAVAILABLE", HTTP_SERVICE_UNAVAILABLE)
            .expect("Failed to set HTTP_SERVICE_UNAVAILABLE");
        module
            .set("HTTP_GATEWAY_TIMEOUT", HTTP_GATEWAY_TIMEOUT)
            .expect("Failed to set HTTP_GATEWAY_TIMEOUT");
        module
            .set("HTTP_VERSION_NOT_SUPPORTED", HTTP_VERSION_NOT_SUPPORTED)
            .expect("Failed to set HTTP_VERSION_NOT_SUPPORTED");
        module
            .set("HTTP_INSUFFICIENT_STORAGE", HTTP_INSUFFICIENT_STORAGE)
            .expect("Failed to set HTTP_INSUFFICIENT_STORAGE");

        // 日志级别常量
        module
            .set("LOG_EMERG", LOG_EMERG)
            .expect("Failed to set LOG_EMERG");
        module
            .set("LOG_ALERT", LOG_ALERT)
            .expect("Failed to set LOG_ALERT");
        module
            .set("LOG_CRIT", LOG_CRIT)
            .expect("Failed to set LOG_CRIT");
        module
            .set("LOG_ERR", LOG_ERR)
            .expect("Failed to set LOG_ERR");
        module
            .set("LOG_WARN", LOG_WARN)
            .expect("Failed to set LOG_WARN");
        module
            .set("LOG_NOTICE", LOG_NOTICE)
            .expect("Failed to set LOG_NOTICE");
        module
            .set("LOG_INFO", LOG_INFO)
            .expect("Failed to set LOG_INFO");
        module
            .set("LOG_DEBUG", LOG_DEBUG)
            .expect("Failed to set LOG_DEBUG");
    }
}

/// 全局 Lua 引擎实例，使用延迟初始化确保线程安全
///
/// 整个应用程序中共享同一个 Lua 引擎实例，避免重复初始化开销
pub static LUA_ENGINE: LazyLock<LuaEngine> = LazyLock::new(LuaEngine::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_engine_creation() {
        // 测试 Lua 引擎是否能够正常创建
        let engine = LuaEngine::new();
        assert!(engine.lua.globals().contains_key("candy").unwrap());
        assert!(engine.lua.globals().contains_key("cd").unwrap());
    }

    #[test]
    fn test_version_info() {
        // 测试系统信息的访问
        let engine = LuaEngine::new();

        let version: String = engine.lua.load("return candy.version").eval().unwrap();
        let name: String = engine.lua.load("return candy.name").eval().unwrap();
        let os: String = engine.lua.load("return candy.os").eval().unwrap();
        let arch: String = engine.lua.load("return candy.arch").eval().unwrap();
        let compiler: String = engine.lua.load("return candy.compiler").eval().unwrap();
        let commit: String = engine.lua.load("return candy.commit").eval().unwrap();

        assert!(!version.is_empty());
        assert!(!name.is_empty());
        assert!(!os.is_empty());
        assert!(!arch.is_empty());
        assert!(!compiler.is_empty());
        assert!(!commit.is_empty());
    }

    #[test]
    fn test_log_function() {
        // 测试 log 函数是否能够正常工作
        let engine = LuaEngine::new();

        // 执行 log 函数应该不会出错
        engine
            .lua
            .load("candy.log('Test log message')")
            .eval::<()>()
            .unwrap();
    }

    #[test]
    fn test_code_cache_operations() {
        // 测试代码缓存操作
        let engine = LuaEngine::new();

        // 测试添加和获取缓存
        let test_script = "test_script.lua";
        let test_content = "return 'test value'";
        let test_checksum = 12345;

        // 检查初始缓存是否为空
        let initial_stats = engine.cache_stats();
        assert_eq!(initial_stats.0, 0);

        // 添加到缓存
        engine.code_cache.insert(
            test_script.to_string(),
            LuaCodeCacheEntry {
                compiled_func: engine.lua.load(test_content).into_function().unwrap(),
                checksum: test_checksum,
            },
        );

        // 检查缓存是否包含条目
        assert!(engine.code_cache.contains_key(test_script));
        let after_insert_stats = engine.cache_stats();
        assert_eq!(after_insert_stats.0, 1);

        // 测试清除特定条目
        let removed = engine.remove_from_cache(test_script);
        assert!(removed);
        assert!(!engine.code_cache.contains_key(test_script));

        // 测试清除所有缓存
        let another_script = "another_script.lua";
        engine.code_cache.insert(
            another_script.to_string(),
            LuaCodeCacheEntry {
                compiled_func: engine
                    .lua
                    .load("return 'another value'")
                    .into_function()
                    .unwrap(),
                checksum: 67890,
            },
        );
        assert!(!engine.code_cache.is_empty());

        engine.clear_cache();
        assert!(engine.code_cache.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        // 测试缓存统计信息
        let engine = LuaEngine::new();

        let (count, estimated_bytes, memory) = engine.cache_stats();
        assert_eq!(count, 0);
        assert_eq!(estimated_bytes, 0);

        // 添加一些条目
        for i in 0..5 {
            let script = format!("script_{}.lua", i);
            engine.code_cache.insert(
                script,
                LuaCodeCacheEntry {
                    compiled_func: engine
                        .lua
                        .load(format!("return 'value{}'", i))
                        .into_function()
                        .unwrap(),
                    checksum: i as u64,
                },
            );
        }

        let (count, estimated_bytes, memory) = engine.cache_stats();
        assert_eq!(count, 5);
        assert!(estimated_bytes > 0);
        assert!(memory > 0);

        // 打印统计信息
        engine.print_cache_stats();
    }

    #[test]
    fn test_shared_dict_init() {
        let engine = LuaEngine::new();

        // 动态初始化共享字典
        engine.init_shared_dict("dogs", 10 * 1024 * 1024);

        // 验证字典已创建
        assert!(engine.shared_dicts.contains_key("dogs"));

        // 在 Lua 中访问
        let result: bool = engine
            .lua
            .load(
                r#"
                local dogs = cd.shared.dogs
                return dogs ~= nil
            "#,
            )
            .eval()
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_shared_dict_get_set() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 在 Lua 中 set 和 get，测试返回值
        let (success, err, forcible, value): (bool, Option<String>, bool, String) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                local succ, err, forcible = test:set("key", "value", 0, 42)
                local val, flags = test:get("key")
                return succ, err, forcible, val
            "#,
            )
            .eval()
            .unwrap();
        assert!(success);
        assert!(err.is_none());
        assert!(!forcible);
        assert_eq!(value, "value");
    }

    #[test]
    fn test_shared_dict_set_with_forcible() {
        let engine = LuaEngine::new();
        // 创建一个很小的字典
        engine.init_shared_dict("small", 100);

        // 测试 LRU 淘汰
        let (success, forcible): (bool, bool) = engine
            .lua
            .load(
                r#"
                local small = cd.shared.small
                -- 填满容量
                small:set("k1", "v1")
                small:set("k2", "v2")
                small:set("k3", "v3")
                -- 添加一个需要淘汰的大条目
                local succ, err, forcible = small:set("big", string.rep("x", 50))
                return succ, forcible
            "#,
            )
            .eval()
            .unwrap();
        assert!(success);
        assert!(forcible);
    }

    #[test]
    fn test_shared_dict_get_not_found() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 获取不存在的键
        let result: mlua::Value = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                local val, flags = test:get("nonexistent")
                return val
            "#,
            )
            .eval()
            .unwrap();
        assert!(matches!(result, mlua::Value::Nil));
    }

    #[test]
    fn test_shared_dict_safe_set_lua() {
        let engine = LuaEngine::new();
        // 创建一个小容量字典
        engine.init_shared_dict("small", 100);

        // 测试 safe_set 在内存不足时返回 nil
        let (ok, err): (Option<bool>, Option<String>) = engine
            .lua
            .load(
                r#"
                local small = cd.shared.small
                -- 填满容量
                small:safe_set("k1", "v1")
                small:safe_set("k2", "v2")
                -- 尝试添加更多（应该失败）
                local ok, err = small:safe_set("k3", "v3")
                return ok, err
            "#,
            )
            .eval()
            .unwrap();
        // ok 应该是 nil（不是 false）
        assert!(ok.is_none());
        assert_eq!(err, Some("no memory".to_string()));
    }

    #[test]
    fn test_shared_dict_safe_set_vs_set_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("small", 100);

        // 对比 set 和 safe_set 的行为
        let (set_forcible, safe_ok): (bool, Option<bool>) = engine
            .lua
            .load(
                r#"
                local small = cd.shared.small

                -- 使用 set 填满容量
                local succ, err, forcible = small:set("k1", "v1")
                small:set("k2", "v2")

                -- set 会淘汰 LRU，返回 forcible=true
                local succ, err, forcible = small:set("k3", "v3")
                local set_forcible = forcible

                -- 清空
                small:flush_all()

                -- 再次填满
                small:safe_set("k1", "v1")
                small:safe_set("k2", "v2")

                -- safe_set 不会淘汰，返回 nil
                local ok, err = small:safe_set("k3", "v3")

                return set_forcible, ok
            "#,
            )
            .eval()
            .unwrap();
        assert!(set_forcible); // set 淘汰了条目
        assert!(safe_ok.is_none()); // safe_set 返回 nil
    }

    #[test]
    fn test_shared_dict_get_stale() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试 get_stale 获取未过期条目
        let (value, flags, stale): (String, i64, bool) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                test:set("key", "value", 0, 42)
                local val, flags, stale = test:get_stale("key")
                return val, flags, stale
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(value, "value");
        assert_eq!(flags, 42);
        assert!(!stale);

        // 测试 get_stale 获取已过期条目
        let (value, stale): (String, bool) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                test:set("expired_key", "expired_value", 0.1, 0)
                -- 等待过期
                os.execute("sleep 0.15")
                -- get 应该返回 nil
                local val1 = test:get("expired_key")
                -- get_stale 应该返回值并标记为过期
                local val2, flags, stale = test:get_stale("expired_key")
                return val2, stale
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(value, "expired_value");
        assert!(stale);
    }

    #[test]
    fn test_shared_dict_add_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试 add 方法
        let (success1, err1, forcible1, success2, err2): (
            bool,
            Option<String>,
            bool,
            bool,
            Option<String>,
        ) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                -- 添加新键
                local succ, err, forcible = test:add("key1", "value1")
                local success1, err1, forcible1 = succ, err, forcible
                -- 再次添加相同键
                local succ, err, forcible = test:add("key1", "value2")
                return success1, err1, forcible1, succ, err
            "#,
            )
            .eval()
            .unwrap();
        assert!(success1);
        assert!(err1.is_none());
        assert!(!forcible1);
        assert!(!success2);
        assert_eq!(err2, Some("exists".to_string()));
    }

    #[test]
    fn test_shared_dict_add_with_lru_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("small", 100);

        // 测试 add 的 LRU 淘汰
        let (success, forcible): (bool, bool) = engine
            .lua
            .load(
                r#"
                local small = cd.shared.small
                -- 填满容量
                small:set("k1", "v1")
                small:set("k2", "v2")
                -- add 新键需要 LRU 淘汰
                local succ, err, forcible = small:add("k3", "v3")
                return succ, forcible
            "#,
            )
            .eval()
            .unwrap();
        assert!(success);
        assert!(forcible);
    }

    #[test]
    fn test_shared_dict_safe_add_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("small", 100);

        // 测试 safe_add
        let (ok1, err1, ok2, err2): (Option<bool>, Option<String>, Option<bool>, Option<String>) =
            engine
                .lua
                .load(
                    r#"
                local small = cd.shared.small
                -- 添加新键
                local ok, err = small:safe_add("k1", "v1")
                local ok1, err1 = ok, err
                -- 再次添加相同键
                local ok, err = small:safe_add("k1", "v2")
                -- 内存不足时不淘汰，返回 nil
                small:safe_add("k2", "v2")
                local ok, err = small:safe_add("k3", "v3")
                return ok1, err1, ok, err
            "#,
                )
                .eval()
                .unwrap();
        assert_eq!(ok1, Some(true));
        assert!(err1.is_none());
        assert!(ok2.is_none()); // 内存不足返回 nil
        assert_eq!(err2, Some("no memory".to_string()));
    }

    #[test]
    fn test_shared_dict_replace_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试 replace 方法
        let (success1, err1, success2, err2, value): (
            bool,
            Option<String>,
            bool,
            Option<String>,
            String,
        ) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                -- 替换不存在的键
                local succ, err, forcible = test:replace("key1", "value1")
                local success1, err1 = succ, err
                -- 设置后替换
                test:set("key1", "value1")
                local succ, err, forcible = test:replace("key1", "value2")
                local success2, err2 = succ, err
                -- 获取值
                local val = test:get("key1")
                return success1, err1, success2, err2, val
            "#,
            )
            .eval()
            .unwrap();
        assert!(!success1);
        assert_eq!(err1, Some("not found".to_string()));
        assert!(success2);
        assert!(err2.is_none());
        assert_eq!(value, "value2");
    }

    #[test]
    fn test_shared_dict_delete_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试 delete 方法
        let (result1, result2): (Option<bool>, Option<bool>) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                test:set("key1", "value1")
                -- 删除存在的键
                local ok = test:delete("key1")
                local result1 = ok
                -- 删除不存在的键
                local ok = test:delete("key1")
                local result2 = ok
                return result1, result2
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(result1, Some(true));
        assert!(result2.is_none()); // OpenResty 返回 nil 而不是 false
    }

    #[test]
    fn test_shared_dict_incr_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试 incr 方法
        let (val1, err1, val2, val4): (Option<i64>, Option<String>, Option<i64>, Option<f64>) =
            engine
                .lua
                .load(
                    r#"
                local test = cd.shared.test
                -- 不存在的键，无初始值
                local val, err, forcible = test:incr("key1", 1)
                local val1, err1 = val, err
                -- 不存在的键，有初始值
                local val, err, forcible = test:incr("key2", 5, 0)
                local val2 = val
                -- 已存在的键
                local val, err, forcible = test:incr("key2", 3)
                -- 浮点数
                local val, err, forcible = test:incr("key2", 0.5)
                local val4 = val
                return val1, err1, val2, val4
            "#,
                )
                .eval()
                .unwrap();
        assert!(val1.is_none());
        assert_eq!(err1, Some("not found".to_string()));
        assert_eq!(val2, Some(5));
        assert_eq!(val4, Some(8.5));
    }

    #[test]
    fn test_shared_dict_incr_with_forcible_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("small", 80);

        // 测试 incr 的 forcible 返回值
        let (forcible1, forcible2, forcible3): (Option<bool>, Option<bool>, Option<bool>) = engine
            .lua
            .load(
                r#"
                local small = cd.shared.small
                -- 填满容量
                small:set("k1", "v1")
                small:set("k2", "v2")
                -- incr 新键需要 LRU 淘汰
                local val, err, forcible = small:incr("k3", 1, 0)
                local forcible1 = forcible
                -- 更新现有键，forcible = false
                local val, err, forcible = small:incr("k3", 1)
                local forcible2 = forcible
                -- 不存在的键无 init，forcible = nil
                local val, err, forcible = small:incr("nonexistent", 1)
                local forcible3 = forcible
                return forcible1, forcible2, forcible3
            "#,
            )
            .eval()
            .unwrap();
        assert!(forcible1.unwrap()); // 创建新键时触发了 LRU
        assert_eq!(forcible2, Some(false)); // 更新现有键，forcible = false
        assert!(forcible3.is_none()); // 不存在的键无 init，forcible = nil
    }

    #[test]
    fn test_shared_dict_lpush_rpush_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试 lpush/rpush
        let (len1, len2, len3, len4): (Option<i64>, Option<i64>, Option<i64>, Option<i64>) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                -- lpush 新列表
                local len, err = test:lpush("mylist", "value1")
                local len1 = len
                -- rpush 添加到尾部
                local len, err = test:rpush("mylist", "value2")
                local len2 = len
                -- lpush 添加到头部
                local len, err = test:lpush("mylist", "value0")
                local len3 = len
                -- llen 获取长度
                local len, err = test:llen("mylist")
                local len4 = len
                return len1, len2, len3, len4
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(len1, Some(1));
        assert_eq!(len2, Some(2));
        assert_eq!(len3, Some(3));
        assert_eq!(len4, Some(3));
    }

    #[test]
    fn test_shared_dict_lpop_rpop_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试 lpop/rpop
        let (val1, val2, val3, val4, len): (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i64>,
        ) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                -- 创建列表: [a, b, c]
                test:rpush("mylist", "a")
                test:rpush("mylist", "b")
                test:rpush("mylist", "c")
                -- lpop 从头部弹出
                local val1, err = test:lpop("mylist")
                -- rpop 从尾部弹出
                local val2, err = test:rpop("mylist")
                -- 弹出最后一个
                local val3, err = test:lpop("mylist")
                -- 空列表弹出
                local val4, err = test:lpop("mylist")
                -- 获取长度
                local len, err = test:llen("mylist")
                return val1, val2, val3, val4, len
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(val1, Some("a".to_string()));
        assert_eq!(val2, Some("c".to_string()));
        assert_eq!(val3, Some("b".to_string()));
        assert!(val4.is_none()); // 空列表返回 nil
        assert_eq!(len, Some(0)); // 键不存在时 llen 返回 0（视为空列表）
    }

    #[test]
    fn test_shared_dict_list_not_a_list_lua() {
        let engine = LuaEngine::new();
        engine.init_shared_dict("test", 1024 * 1024);

        // 测试对非列表值操作
        let (err1, err2): (Option<String>, Option<String>) = engine
            .lua
            .load(
                r#"
                local test = cd.shared.test
                -- 设置标量值
                test:set("key", "value")
                -- 尝试 lpush 到标量
                local len, err1 = test:lpush("key", "item")
                -- 尝试 lpop 标量
                local val, err2 = test:lpop("key")
                return err1, err2
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(err1, Some("value not a list".to_string()));
        assert_eq!(err2, Some("value not a list".to_string()));
    }
}

