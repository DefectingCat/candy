use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use mlua::{Function, Lua};
use tracing::{debug, info};

use crate::consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};

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

/// Lua 引擎实例，包含 Lua 虚拟机和代码缓存
pub struct LuaEngine {
    /// Lua 虚拟机实例
    pub lua: Lua,
    /// Lua 代码缓存，用于存储编译后的 Lua 脚本
    /// 键：脚本文件路径
    /// 值：(编译后的函数, 脚本内容的校验和)
    pub code_cache: Arc<DashMap<String, LuaCodeCacheEntry>>,
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

        // 创建主模块
        let module = lua.create_table().expect("Failed to create Lua module");

        // 注册日志函数
        Self::register_log_function(&lua, &module);

        // 注册版本信息常量
        Self::register_version_info(&module);

        // 注册 HTTP 常量
        Self::register_http_constants(&module);

        // 将 `candy` 模块设置为全局变量
        lua.globals()
            .set("candy", module)
            .expect("设置全局变量 candy 失败");

        Self { lua, code_cache }
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
}
