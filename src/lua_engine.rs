use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use mlua::Lua;
use tracing::info;

use crate::consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION};

pub struct LuaEngine {
    pub lua: Lua,
    /// Lua 共享字典
    #[allow(dead_code)]
    pub shared_table: Arc<DashMap<String, String>>,
}
impl LuaEngine {
    pub fn new() -> Self {
        let lua = Lua::new();
        let shared_table: DashMap<String, String> = DashMap::new();
        let shared_table = Arc::new(shared_table);

        let module = lua.create_table().expect("创建表失败");
        let shared_api = lua.create_table().expect("创建共享表失败");

        // 在 Lua 中创建共享字典
        let shared_table_get = shared_table.clone();
        shared_api
            .set(
                "set",
                lua.create_function(move |_, (key, value): (String, String)| {
                    shared_table_get.insert(key, value.clone());
                    Ok(())
                })
                .expect("创建 set 函数失败"),
            )
            .expect("设置失败");
        let shared_table_get = shared_table.clone();
        shared_api
            .set(
                "get",
                lua.create_function(move |_, key: String| {
                    let value = shared_table_get.get(&key);
                    match value {
                        Some(value) => Ok(value.clone()),
                        None => {
                            tracing::error!("shared_api: 获取的键不存在: {}", key);
                            Ok(String::new())
                        }
                    }
                })
                .expect("创建 get 函数失败"),
            )
            .expect("获取失败");
        module
            .set("shared", shared_api)
            .expect("设置 shared_api 失败");

        // 日志函数
        module
            .set(
                "log",
                lua.create_function(move |_, msg: String| {
                    info!("Lua: {}", msg);
                    Ok(())
                })
                .expect("创建 log 函数失败"),
            )
            .expect("设置 log 失败");

        module.set("version", VERSION).expect("设置 version 失败");
        module.set("name", NAME).expect("设置 name 失败");
        module.set("os", OS).expect("设置 os 失败");
        module.set("arch", ARCH).expect("设置 arch 失败");
        module
            .set("compiler", COMPILER)
            .expect("设置 compiler 失败");
        module.set("commit", COMMIT).expect("设置 commit 失败");

        // 全局变量 candy
        lua.globals()
            .set("candy", module)
            .expect("将 candy 表设置到 Lua 引擎失败");

        Self { lua, shared_table }
    }
}
/// Lua 脚本执行器
pub static LUA_ENGINE: LazyLock<LuaEngine> = LazyLock::new(LuaEngine::new);
