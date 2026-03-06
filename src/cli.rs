//! 命令行参数解析模块
//!
//! 提供命令行参数的解析和配置功能。
//!
//! # 支持的参数
//!
//! - `-c, --config <FILE>`：指定配置文件路径（默认：./config.toml）
//! - `-h, --help`：显示帮助信息
//! - `-V, --version`：显示版本信息
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
//!
//! # 查看版本信息
//! candy --version
//! ```
//!
//! # 配置文件
//!
//! 默认配置文件路径为 `./config.toml`。
//! 如果配置文件不存在或格式错误，程序会输出错误信息并退出。

use clap::Parser;

/// Candy Web 服务器命令行参数
///
/// 使用 `clap` 库解析命令行参数，支持配置文件路径指定。
#[derive(Parser)]
#[command(version, about, long_about = Some("A modern, lightweight web server written in Rust.\n\nFeatures:\n- Static file serving with directory listing support\n- Reverse proxying to backend servers\n- Lua scripting (optional feature)\n- SSL/TLS encryption (HTTPS)\n- HTTP/2 support\n- Auto-reload config on file change\n- Multiple virtual hosts\n- Single binary deployment"))]
pub struct Cli {
    /// 配置文件路径
    ///
    /// 指定 Candy 服务器的配置文件位置。
    /// 支持相对路径和绝对路径。
    ///
    /// # 默认值
    ///
    /// 默认为 `./config.toml`
    ///
    /// # 示例
    ///
    /// ```bash
    /// candy -c /etc/candy/config.toml
    /// candy --config ./my-config.toml
    /// ```
    #[arg(short, long, value_name = "FILE", default_value = "./config.toml")]
    pub config: String,
}
