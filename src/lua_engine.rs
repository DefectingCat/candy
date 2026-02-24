use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use mlua::Lua;
use tracing::{error, info};

use crate::consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};

/// Lua 引擎实例，包含 Lua 虚拟机和共享字典
pub struct LuaEngine {
    /// Lua 虚拟机实例
    pub lua: Lua,
    /// 线程安全的共享字典，用于 Lua 和 Rust 之间交换数据
    #[allow(dead_code)]
    pub shared_table: Arc<DashMap<String, String>>,
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
    /// - 共享字典操作 (shared.set/get)
    /// - 日志功能 (log)
    /// - 版本信息访问 (version/name/os/arch/compiler/commit)
    pub fn new() -> Self {
        let lua = Lua::new();
        let shared_table = Arc::new(DashMap::new());

        // 创建主模块和共享API子模块
        let module = lua.create_table().expect("Failed to create Lua module");
        let shared_api = lua
            .create_table()
            .expect("Failed to create shared API submodule");

        // 注册共享字典操作方法
        Self::register_shared_api(&lua, &module, &shared_api, shared_table.clone());

        // 注册日志函数
        Self::register_log_function(&lua, &module);

        // 注册版本信息常量
        Self::register_version_info(&module);

        // 将 `candy` 模块设置为全局变量
        lua.globals()
            .set("candy", module)
            .expect("设置全局变量 candy 失败");

        Self { lua, shared_table }
    }

    /// 注册共享字典操作 API
    fn register_shared_api(
        lua: &Lua,
        module: &mlua::Table,
        shared_api: &mlua::Table,
        shared_table: Arc<DashMap<String, String>>,
    ) {
        // 注册 set 方法
        let table_clone = shared_table.clone();
        let set_func = lua
            .create_function(move |_, (key, value): (String, String)| {
                table_clone.insert(key, value.clone());
                Ok(())
            })
            .expect("Failed to create shared dictionary set function");
        shared_api
            .set("set", set_func)
            .expect("Failed to set shared dictionary set method");

        // 注册 get 方法
        let table_clone = shared_table.clone();
        let get_func = lua
            .create_function(move |_, key: String| match table_clone.get(&key) {
                Some(value) => Ok(value.clone()),
                None => {
                    error!("shared_api: Key not found: {}", key);
                    Ok(String::new())
                }
            })
            .expect("Failed to create shared dictionary get function");
        shared_api
            .set("get", get_func)
            .expect("Failed to set shared dictionary get method");

        // 将共享API添加到主模块
        module
            .set("shared", shared_api.clone())
            .expect("Failed to set shared submodule");
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
    fn test_shared_table_operations() {
        // 测试共享字典的 set 和 get 方法
        let engine = LuaEngine::new();
        let key = "test_key";
        let value = "test_value";

        // 使用 Lua 脚本设置和获取值
        let result: String = engine
            .lua
            .load(format!(
                "candy.shared.set('{}', '{}'); return candy.shared.get('{}')",
                key, value, key
            ))
            .eval()
            .unwrap();

        assert_eq!(result, value);

        // 测试获取不存在的键
        let result: String = engine
            .lua
            .load("return candy.shared.get('nonexistent_key')")
            .eval()
            .unwrap();

        assert_eq!(result, "");
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
}