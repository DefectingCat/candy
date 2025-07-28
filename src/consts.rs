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
