use std::{collections::BTreeMap, env, process::exit, sync::OnceLock};

use tracing::error;

use crate::config::{init_config, Settings};

// global settings
static SETTINGS: OnceLock<Settings> = OnceLock::new();
pub fn get_settings() -> &'static Settings {
    SETTINGS.get_or_init(|| {
        init_config()
            .map_err(|err| {
                error!("get_or_init config failed: {err}");
                exit(1);
            })
            .unwrap()
    })
}

// pre defined
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const OS: &str = env::consts::OS;
pub const ARCH: &str = env::consts::ARCH;

// config defaults
pub const HOST_INDEX: [&str; 1] = ["index.html"];
pub fn host_index() -> Vec<String> {
    HOST_INDEX.map(|h| h.to_string()).to_vec()
}

// default http keep alive timeout
pub const KEEP_ALIVE_TIMEOUTD_EFAULT: u16 = 75;
pub fn keep_alive_timeoutd_efault() -> u16 {
    KEEP_ALIVE_TIMEOUTD_EFAULT
}

// default process for single http request
pub const PROCESS_TIMEOUT: u16 = 15;
pub fn process_timeout() -> u16 {
    PROCESS_TIMEOUT
}

// default mime type for unknow file
pub const MIME_DEFAULT: &str = "application/octet-stream";
pub fn mime_default() -> String {
    MIME_DEFAULT.to_string()
}

// default mime types
pub fn types_default() -> BTreeMap<String, String> {
    BTreeMap::new()
}
macro_rules! insert_mime {
    ($name:literal, $mime:ident, $map:ident) => {
        $map.entry($name.to_string()).or_insert($mime.to_string());
    };
}
pub fn insert_default_mimes(map: &mut BTreeMap<String, String>) {
    use crate::http::mime::*;

    insert_mime!("html", TEXT_HTML, map);
    insert_mime!("htm", TEXT_HTML, map);
    insert_mime!("shtml", TEXT_HTML, map);
    insert_mime!("css", TEXT_CSS, map);
    insert_mime!("xml", TEXT_XML, map);
    insert_mime!("rss", TEXT_XML, map);
    insert_mime!("txt", TEXT_PLAIN, map);

    insert_mime!("gif", IMAGE_GIF, map);
    insert_mime!("jpg", IMAGE_JPEG, map);
    insert_mime!("jpeg", IMAGE_JPEG, map);
    insert_mime!("png", IMAGE_PNG, map);
    insert_mime!("ico", IMAGE_ICON, map);
    insert_mime!("jng", IMAGE_JNG, map);
    insert_mime!("wbmp", IMAGE_WBMP, map);

    insert_mime!("js", APPLICATION_JAVASCRIPT, map);
    insert_mime!("wasm", APPLICATION_WASM, map);
}
