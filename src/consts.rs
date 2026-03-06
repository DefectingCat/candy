//! 常量定义模块
//!
//! 定义服务器使用的所有常量，包括版本信息、构建信息和配置默认值。
//!
//! # 预定义常量
//!
//! - `NAME`：服务器名称
//! - `VERSION`：版本号
//! - `OS`：操作系统
//! - `ARCH`：CPU 架构
//! - `COMPILER`：编译器信息
//! - `COMMIT`：Git 提交哈希
//!
//! # 配置默认值
//!
//! - HTTP 超时时间：75 秒
//! - 上游超时时间：5 秒
//! - 日志级别：info
//! - 日志目录：./logs
//! - 压缩级别：6（平衡速度和压缩率）
//!
//! # 使用示例
//!
//! ```no_run
//! use candy::consts::{NAME, VERSION, timeout_default};
//!
//! println!("{} v{}", NAME, VERSION);
//! println!("Default timeout: {}s", timeout_default());
//! ```

//! 常量定义模块
//!
//! 定义服务器使用的所有常量，包括版本信息、构建信息和配置默认值。
//!
//! # 预定义常量
//!
//! - `NAME`：服务器名称
//! - `VERSION`：版本号
//! - `OS`：操作系统
//! - `ARCH`：CPU 架构
//! - `COMPILER`：编译器信息
//! - `COMMIT`：Git 提交哈希
//!
//! # 配置默认值
//!
//! - HTTP 超时时间：75 秒
//! - 上游超时时间：5 秒
//! - 日志级别：info
//! - 日志目录：./logs
//! - 压缩级别：6（平衡速度和压缩率）
//!
//! # 使用示例
//!
//! ```no_run
//! use candy::consts::{NAME, VERSION, timeout_default};
//!
//! println!("{} v{}", NAME, VERSION);
//! println!("Default timeout: {}s", timeout_default());
//! ```

use std::env;

// ========== 预定义常量 ==========

/// 服务器名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 服务器版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 操作系统名称
pub const OS: &str = env::consts::OS;

/// CPU 架构
pub const ARCH: &str = env::consts::ARCH;

/// 编译器信息
pub const COMPILER: &str = env!("RUA_COMPILER");

/// Git 提交哈希
pub const COMMIT: &str = env!("RUA_COMMIT");

// ========== 配置默认值 ==========

/// 默认索引文件列表
pub const HOST_INDEX: [&str; 1] = ["index.html"];

/// 返回默认的索引文件列表
///
/// # 返回值
///
/// 返回默认索引文件列表（包含 "index.html"）。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::host_index;
///
/// let index_files = host_index();
/// assert_eq!(index_files, vec!["index.html"]);
/// ```
pub fn host_index() -> Vec<String> {
    HOST_INDEX.map(|h| h.to_string()).to_vec()
}

/// 默认 HTTP 连接超时时间（秒）
pub const TIMEOUT_EFAULT: u16 = 75;

/// 返回默认的 HTTP 连接超时时间
///
/// 默认超时时间为 75 秒，符合 HTTP 标准的推荐值。
///
/// # 返回值
///
/// 返回默认超时时间（75 秒）。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::timeout_default;
///
/// let timeout = timeout_default();
/// assert_eq!(timeout, 75);
/// ```
pub fn timeout_default() -> u16 {
    TIMEOUT_EFAULT
}

/// 默认上游服务器连接超时时间（秒）
pub const UPSTREAM_TIMEOUT: u16 = 5;

/// 返回默认的上游服务器连接超时时间
///
/// 上游服务器连接超时默认为 5 秒。
///
/// # 返回值
///
/// 返回默认上游超时时间（5 秒）。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::upstream_timeout_default;
///
/// let timeout = upstream_timeout_default();
/// assert_eq!(timeout, 5);
/// ```
pub fn upstream_timeout_default() -> u16 {
    UPSTREAM_TIMEOUT
}

/// 返回默认的禁用状态
///
/// 用于配置项的默认布尔值，默认为 false。
///
/// # 返回值
///
/// 返回 false。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::default_disabled;
///
/// assert_eq!(default_disabled(), false);
/// ```
pub fn default_disabled() -> bool {
    false
}

/// 默认日志级别
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// 返回默认的日志级别
///
/// 默认日志级别为 "info"。
///
/// # 返回值
///
/// 返回默认日志级别字符串（"info"）。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::default_log_level;
///
/// let level = default_log_level();
/// assert_eq!(level, "info");
/// ```
pub fn default_log_level() -> String {
    DEFAULT_LOG_LEVEL.to_string()
}

/// 默认日志文件夹路径
pub const DEFAULT_LOG_FOLDER: &str = "./logs";

/// 返回默认的日志文件夹路径
///
/// 默认日志目录为 "./logs"。
///
/// # 返回值
///
/// 返回默认日志目录字符串（"./logs"）。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::default_log_folder;
///
/// let folder = default_log_folder();
/// assert_eq!(folder, "./logs");
/// ```
pub fn default_log_folder() -> String {
    DEFAULT_LOG_FOLDER.to_string()
}

/// 返回默认的压缩启用状态
///
/// 默认启用压缩。
///
/// # 返回值
///
/// 返回 true。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::default_compression_enabled;
///
/// assert_eq!(default_compression_enabled(), true);
/// ```
pub fn default_compression_enabled() -> bool {
    true
}

/// 返回默认的压缩级别
///
/// 默认压缩级别为 6，在压缩率和速度之间取得平衡。
///
/// # 压缩级别说明
///
/// - 1：最快压缩，压缩率最低
/// - 6：默认值，平衡速度和压缩率
/// - 9：最佳压缩，速度最慢
///
/// # 返回值
///
/// 返回默认压缩级别（6）。
///
/// # 示例
///
/// ```no_run
/// use candy::consts::default_compression_level;
///
/// let level = default_compression_level();
/// assert_eq!(level, 6);
/// ```
pub fn default_compression_level() -> u8 {
    6 // tower-http default is typically around 6 (middle ground)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_values() {
        // 测试预定义常量
        assert!(!NAME.is_empty());
        assert!(!VERSION.is_empty());
        assert!(!OS.is_empty());
        assert!(!ARCH.is_empty());
        assert!(!COMPILER.is_empty());
        assert!(!COMMIT.is_empty());
    }

    #[test]
    fn test_host_index() {
        // 测试主机索引函数
        let index = host_index();
        assert_eq!(index.len(), 1);
        assert_eq!(index[0], "index.html");
    }

    #[test]
    fn test_timeout_default() {
        // 测试默认超时函数
        assert_eq!(timeout_default(), TIMEOUT_EFAULT);
        assert_eq!(timeout_default(), 75);
    }

    #[test]
    fn test_upstream_timeout_default() {
        // 测试上游超时函数
        assert_eq!(upstream_timeout_default(), UPSTREAM_TIMEOUT);
        assert_eq!(upstream_timeout_default(), 5);
    }

    #[test]
    fn test_default_disabled() {
        // 测试默认禁用值
        assert!(!default_disabled());
    }

    #[test]
    fn test_default_log_level() {
        // 测试默认日志级别
        assert_eq!(default_log_level(), DEFAULT_LOG_LEVEL);
        assert_eq!(default_log_level(), "info");
    }

    #[test]
    fn test_default_log_folder() {
        // 测试默认日志文件夹
        assert_eq!(default_log_folder(), DEFAULT_LOG_FOLDER);
        assert_eq!(default_log_folder(), "./logs");
    }
}
