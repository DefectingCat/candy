//! 错误处理模块
//!
//! 定义 Candy 服务器的错误类型和错误处理机制。
//!
//! # 错误类型
//!
//! 服务器可能遇到的错误类型包括：
//!
//! - **IoError**：文件读写、网络操作等 I/O 错误
//! - **TomlDecode**：配置文件 TOML 解析错误
//! - **Http**：HTTP 协议相关错误
//! - **Time**：系统时间相关错误
//! - **TryFromInt**：整数转换错误
//! - **ToStr**：HTTP 头部字符串转换错误
//! - **InvalidUri**：URL 解析错误
//! - **HyperError**：Hyper 库错误
//! - **Any**：通用错误（anyhow::Error）
//!
//! # 错误处理示例
//!
//! ```no_run
//! use candy::config::Settings;
//!
//! match Settings::new("config.toml") {
//!     Ok(settings) => println!("配置加载成功"),
//!     Err(e) => eprintln!("错误: {:?}", e),
//! }
//! ```

use std::{io, num::TryFromIntError, time::SystemTimeError};

use http::uri::InvalidUri;
use hyper::header::ToStrError;

/// Candy 服务器错误类型
///
/// 定义了服务器运行过程中可能遇到的所有错误类型。
/// 使用 `thiserror` 库实现，支持自动错误转换和友好的错误信息。
///
/// # 错误变体
///
/// 每个错误变体都对应一种特定的错误场景：
///
/// - `Io` - I/O 错误，如文件读取失败、网络连接失败
/// - `TomlDecode` - TOML 配置文件解析错误
/// - `Http` - HTTP 协议错误
/// - `Time` - 系统时间相关错误
/// - `TryFromInt` - 整数类型转换错误
/// - `ToStr` - HTTP 头部字符串转换错误
/// - `InvalidUri` - URL 解析错误
/// - `HyperError` - Hyper HTTP 库错误
/// - `Any` - 通用错误，包装 anyhow::Error
///
/// # 示例
///
/// ```no_run
/// use candy::error::Error;
/// use std::io;
///
/// // 错误会自动转换为 Error 类型
/// let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
/// let err: Error = io_err.into();
///
/// println!("错误信息: {}", err);
/// ```
#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// I/O 错误
    ///
    /// 包括文件读写、网络连接、端口绑定等操作系统级别的错误。
    ///
    /// # 示例场景
    ///
    /// - 配置文件不存在或无法读取
    /// - SSL 证书文件权限错误
    /// - 端口已被占用
    /// - 网络连接失败
    #[error("failed io {0}")]
    Io(#[from] io::Error),

    /// TOML 解析错误
    ///
    /// 配置文件 TOML 格式错误，如语法错误、字段类型不匹配等。
    #[error("failed to decode toml {0}")]
    TomlDecode(#[from] toml::de::Error),

    /// HTTP 协议错误
    ///
    /// HTTP 协议相关错误，如请求格式错误、响应构建失败等。
    #[error("failed to handle http {0}")]
    Http(#[from] http::Error),

    /// 系统时间错误
    ///
    /// 系统时间相关错误，如时间计算溢出等。
    #[error("failed to handle system time {0}")]
    Time(#[from] SystemTimeError),

    /// 整数转换错误
    ///
    /// 整数类型转换失败，如溢出或范围不匹配。
    #[error("failed to convert int {0}")]
    TryFromInt(#[from] TryFromIntError),

    /// 字符串转换错误
    ///
    /// HTTP 头部值转换为字符串失败。
    #[error("failed to convert str {0}")]
    ToStr(#[from] ToStrError),

    /// URL 解析错误
    ///
    /// URL 格式错误或解析失败。
    #[error("failed to convert url {0}")]
    InvalidUri(#[from] InvalidUri),

    /// Hyper 库错误
    ///
    /// Hyper HTTP 库产生的错误，如连接断开、请求超时等。
    #[error("hyper {0}")]
    HyperError(#[from] hyper::Error),

    /// 通用错误
    ///
    /// 包装 anyhow::Error，用于处理其他类型的错误。
    #[error("internal server error {0}")]
    Any(#[from] anyhow::Error),
}

/// Candy 服务器 Result 类型别名
///
/// 使用 `anyhow::Result<T, Error>` 简化函数签名。
/// 这是项目中所有可能失败的操作的标准返回类型。
///
/// # 示例
///
/// ```no_run
/// use candy::error::Result;
///
/// fn load_config() -> Result<String> {
///     // 操作可能失败
///     Ok("config content".to_string())
/// }
///
/// match load_config() {
///     Ok(config) => println!("配置: {}", config),
///     Err(e) => eprintln!("错误: {:?}", e),
/// }
/// ```
pub type Result<T, E = Error> = anyhow::Result<T, E>;
