use std::env;

// pre defined
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const OS: &str = env::consts::OS;
pub const ARCH: &str = env::consts::ARCH;
pub const COMPILER: &str = env!("RUA_COMPILER");
pub const COMMIT: &str = env!("RUA_COMMIT");

// config defaults
pub const HOST_INDEX: [&str; 1] = ["index.html"];
pub fn host_index() -> Vec<String> {
    HOST_INDEX.map(|h| h.to_string()).to_vec()
}

// default http connection timeout
pub const TIMEOUT_EFAULT: u16 = 75;
pub fn timeout_default() -> u16 {
    TIMEOUT_EFAULT
}

// default reverse proxy upstream timeout
pub const UPSTREAM_TIMEOUT: u16 = 5;
pub fn upstream_timeout_default() -> u16 {
    UPSTREAM_TIMEOUT
}

// default boolean false
pub fn default_disabled() -> bool {
    false
}

// default log level
pub const DEFAULT_LOG_LEVEL: &str = "info";
pub fn default_log_level() -> String {
    DEFAULT_LOG_LEVEL.to_string()
}

// default log folder
pub const DEFAULT_LOG_FOLDER: &str = "./logs";
pub fn default_log_folder() -> String {
    DEFAULT_LOG_FOLDER.to_string()
}

// default compression settings
pub fn default_compression_enabled() -> bool {
    true
}

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
