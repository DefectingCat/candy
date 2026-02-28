use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use mlua::{Function, Lua};
use tracing::{debug, info};

use crate::consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};

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
