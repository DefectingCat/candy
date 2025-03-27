use std::{borrow::Cow, collections::BTreeMap, env};

use crate::config::MIMEType;

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

// default mime type for unknow file
pub const MIME_DEFAULT: &str = "application/octet-stream";
pub fn mime_default() -> Cow<'static, str> {
    MIME_DEFAULT.into()
}

// default reverse proxy upstream timeout
pub const UPSTREAM_TIMEOUT: u16 = 5;
pub fn upstream_timeout_default() -> u16 {
    UPSTREAM_TIMEOUT
}

// default mime types
pub fn types_default() -> MIMEType {
    BTreeMap::new()
}
macro_rules! insert_mime {
    ($name:literal, $mime:ident, $map:ident) => {
        $map.entry($name.into()).or_insert($mime.into());
    };
}
