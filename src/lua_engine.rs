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
        let module = lua.create_table().expect("创建 Lua 模块失败");
        let shared_api = lua.create_table().expect("创建共享API子模块失败");

        // 注册共享字典操作方法
        Self::register_shared_api(&lua, &shared_api, shared_table.clone());

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
            .expect("创建共享字典 set 函数失败");
        shared_api
            .set("set", set_func)
            .expect("设置共享字典 set 方法失败");

        // 注册 get 方法
        let table_clone = shared_table.clone();
        let get_func = lua
            .create_function(move |_, key: String| match table_clone.get(&key) {
                Some(value) => Ok(value.clone()),
                None => {
                    error!("shared_api: 获取的键不存在: {}", key);
                    Ok(String::new())
                }
            })
            .expect("创建共享字典 get 函数失败");
        shared_api
            .set("get", get_func)
            .expect("设置共享字典 get 方法失败");

        // 将共享API添加到主模块
        shared_api
            .lua()
            .globals()
            .get::<_, mlua::Table>("candy")
            .expect("获取全局变量 candy 失败")
            .set("shared", shared_api.clone())
            .expect("设置 shared 子模块失败");
    }

    /// 注册日志函数到主模块
    fn register_log_function(lua: &Lua, module: &mlua::Table) {
        let log_func = lua
            .create_function(move |_, msg: String| {
                info!("Lua: {}", msg);
                Ok(())
            })
            .expect("创建日志函数失败");
        module.set("log", log_func).expect("设置 log 方法失败");
    }

    /// 注册版本信息常量到主模块
    fn register_version_info(module: &mlua::Table) {
        module.set("version", VERSION).expect("设置版本号失败");
        module.set("name", NAME).expect("设置应用名称失败");
        module.set("os", OS).expect("设置操作系统信息失败");
        module.set("arch", ARCH).expect("设置架构信息失败");
        module
            .set("compiler", COMPILER)
            .expect("设置编译器信息失败");
        module.set("commit", COMMIT).expect("设置提交哈希失败");
    }
}

/// 全局 Lua 引擎实例，使用延迟初始化确保线程安全
///
/// 整个应用程序中共享同一个 Lua 引擎实例，避免重复初始化开销
pub static LUA_ENGINE: LazyLock<LuaEngine> = LazyLock::new(LuaEngine::new);
