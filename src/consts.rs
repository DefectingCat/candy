use std::env;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const OS: &str = env::consts::OS;
pub const ARCH: &str = env::consts::ARCH;

// config defaults
pub const KEEP_ALIVE_TIMEOUTD_EFAULT: u16 = 75;
pub fn keep_alive_timeoutd_efault() -> u16 {
    KEEP_ALIVE_TIMEOUTD_EFAULT
}
