//! Candy Web 服务器主程序
//!
//! Candy 是一个高性能、易配置的 Web 服务器，支持静态文件服务、反向代理、负载均衡等功能。
//!
//! # 功能特性
//!
//! - 静态文件服务，支持目录列表
//! - 反向代理，支持多种负载均衡算法（轮询、加权轮询、IP 哈希、最少连接）
//! - 正向代理支持
//! - Lua 脚本支持（可选）
//! - SSL/TLS 加密和 HTTP/2
//! - 配置文件自动重载（带防抖机制）
//! - 响应压缩（gzip、deflate、brotli、zstd）
//!
//! # 启动流程
//!
//! 1. 解析命令行参数，获取配置文件路径
//! 2. 加载并验证配置文件
//! 3. 初始化日志系统（文件日志 + 控制台输出）
//! 4. 加载上游服务器配置
//! 5. 初始化 Lua 共享字典（如果启用）
//! 6. 启动初始服务器实例
//! 7. 启动配置文件监听器，支持热重载
//! 8. 等待关闭信号（Ctrl+C）
//! 9. 优雅关闭所有服务器
//!
//! # 使用示例
//!
//! ```bash
//! # 使用默认配置文件
//! candy
//!
//! # 使用指定配置文件
//! candy -c /path/to/config.toml
//!
//! # 查看帮助信息
//! candy --help
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use crate::config::Settings;
use crate::http::{handle_config_change, load_upstreams, start_initial_servers};
use crate::utils::{initialize_logger, shutdown_application, start_config_watcher};

#[cfg(feature = "lua")]
use crate::lua_engine::LUA_ENGINE;

use mimalloc::MiMalloc;

/// 全局内存分配器
///
/// 使用 mimalloc 作为全局内存分配器，提供更好的内存分配性能。
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

/// 程序主入口
///
/// 执行 Candy 服务器的完整启动流程，包括配置加载、服务器初始化和运行。
///
/// # 启动步骤
///
/// 1. **解析命令行参数**：获取配置文件路径和其他选项
/// 2. **加载配置文件**：读取并验证 TOML 配置文件
/// 3. **初始化日志系统**：配置文件日志和控制台输出
/// 4. **加载上游服务器**：初始化负载均衡配置
/// 5. **初始化 Lua 引擎**（如果启用）：设置共享字典等
/// 6. **启动服务器实例**：根据配置启动 HTTP/HTTPS 服务器
/// 7. **配置热重载**：监听配置文件变更并自动重载
/// 8. **等待关闭信号**：监听 Ctrl+C 信号
/// 9. **优雅关闭**：停止所有服务器并清理资源
///
/// # 错误处理
///
/// 如果配置文件不存在、格式错误、验证失败或服务器启动失败，
/// 程序会输出详细的错误信息并退出。
///
/// # 示例
///
/// ```no_run
/// // 启动服务器（使用默认配置文件 config.toml）
/// // candy
///
/// // 使用指定配置文件
/// // candy -c /path/to/config.toml
/// ```
///
/// # 注意
///
/// - 配置文件默认路径为 `config.toml`
/// - 支持 HTTP 和 HTTPS 服务器同时运行
/// - 配置文件变更会自动触发服务器重载（带防抖）
#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let args = cli::Cli::parse();

    // 加载和验证配置
    //
    // 从指定的配置文件路径加载 TOML 配置，并执行以下验证：
    // - 配置文件格式正确性
    // - 上游服务器配置完整性
    // - SSL 证书文件存在性
    // - 路由配置有效性
    let settings =
        Settings::new(&args.config).with_context(|| "Failed to initialize configuration")?;

    // 初始化日志系统
    //
    // 根据配置初始化日志系统，支持：
    // - 文件日志：按日期滚动，存储在配置的日志目录中
    // - 控制台输出：实时显示日志信息
    // - 可配置的日志级别：trace, debug, info, warn, error
    initialize_logger(&settings).await?;

    // 加载上游服务器配置
    //
    // 将配置文件中的上游服务器组加载到全局存储中，
    // 并初始化健康检查任务（如果配置了主动健康检查）。
    load_upstreams(&settings);

    // 初始化 Lua 共享字典（如果启用了 Lua 特性）
    //
    // 根据 lua_shared_dict 配置初始化共享内存区域，
    // 用于在 Lua 脚本中共享数据。
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

    // 启动初始服务器实例
    //
    // 根据配置文件中的主机配置启动所有服务器实例：
    // - HTTP 服务器（普通端口）
    // - HTTPS 服务器（启用 SSL 的端口）
    // 返回所有服务器句柄，用于后续管理和优雅关闭。
    let handles = start_initial_servers(settings).await?;

    // 启动配置文件监听器
    //
    // 监听配置文件变更，当检测到变更时：
    // 1. 重新加载配置文件
    // 2. 验证新配置
    // 3. 优雅关闭所有旧服务器
    // 4. 启动新服务器实例
    let handles_clone = handles.clone();
    let stop_tx = start_config_watcher(&args.config, move |result| {
        let handles_clone = handles_clone.clone();
        Box::pin(handle_config_change(result, handles_clone))
    })?;

    info!("Server started");

    // 保持主线程运行，直到收到停止信号（Ctrl+C）
    //
    // 当用户按下 Ctrl+C 时，程序会收到中断信号，
    // 然后执行优雅关闭流程。
    tokio::signal::ctrl_c().await?;

    // 优雅关闭应用程序
    //
    // 执行以下清理操作：
    // 1. 停止配置文件监听器
    // 2. 对所有服务器发送优雅关闭信号
    // 3. 等待所有正在处理的请求完成（最多 30 秒）
    // 4. 清理资源并退出
    shutdown_application(handles, stop_tx).await;

    Ok(())
}
