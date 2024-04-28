use std::env;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const OS: &str = env::consts::OS;
pub const ARCH: &str = env::consts::ARCH;

// config defaults
pub const HOST_INDEX: [&str; 1] = ["index.html"];
pub fn host_index() -> Vec<String> {
    HOST_INDEX.map(|h| h.to_string()).to_vec()
}

pub const KEEP_ALIVE_TIMEOUTD_EFAULT: u16 = 75;
pub fn keep_alive_timeoutd_efault() -> u16 {
    KEEP_ALIVE_TIMEOUTD_EFAULT
}

pub const PROCESS_TIMEOUT: u16 = 75;
pub fn process_timeout() -> u16 {
    PROCESS_TIMEOUT
}
