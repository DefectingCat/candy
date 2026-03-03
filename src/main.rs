use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use crate::config::Settings;
use crate::http::{handle_config_change, load_upstreams, start_initial_servers};
use crate::utils::{initialize_logger, shutdown_application, start_config_watcher};

#[cfg(feature = "lua")]
use crate::lua_engine::LUA_ENGINE;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod cli;
mod config;
mod consts;
mod error;
mod http;
#[cfg(feature = "lua")]
mod lua_engine;
mod middlewares;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    // 加载和验证配置
    let settings =
        Settings::new(&args.config).with_context(|| "Failed to initialize configuration")?;

    // 初始化日志系统
    initialize_logger(&settings).await?;

    // 加载上游服务器配置
    load_upstreams(&settings);

    // 初始化 Lua 共享字典
    #[cfg(feature = "lua")]
    {
        if let Some(dicts) = &settings.lua_shared_dict {
            for dict_config in dicts {
                if let Ok(capacity) = dict_config.parse_size() {
                    LUA_ENGINE.init_shared_dict(&dict_config.name, capacity);
                }
            }
        }
    }

    // 启动初始服务器
    let handles = start_initial_servers(settings).await?;

    // 启动配置文件监听
    let handles_clone = handles.clone();
    let stop_tx = start_config_watcher(&args.config, move |result| {
        let handles_clone = handles_clone.clone();
        Box::pin(handle_config_change(result, handles_clone))
    })?;

    info!("Server started");

    // 保持主线程运行，直到收到停止信号
    tokio::signal::ctrl_c().await?;

    // 优雅关闭应用程序
    shutdown_application(handles, stop_tx).await;

    Ok(())
}
