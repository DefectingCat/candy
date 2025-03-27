use std::{borrow::Cow, collections::BTreeMap, env, sync::OnceLock};

use crate::{
    config::{MIMEType, Settings},
    error::{Error, Result},
};

// global settings
pub static SETTINGS: OnceLock<Settings> = OnceLock::new();
pub fn get_settings() -> Result<&'static Settings> {
    SETTINGS.get().ok_or(Error::Empty)
}

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
pub fn insert_default_mimes(map: &mut MIMEType) {
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
    insert_mime!("webp", IMAGE_WEBP, map);
    insert_mime!("avif", IMAGE_AVIF, map);
    insert_mime!("svg", IMAGE_SVG, map);
    insert_mime!("svgz", IMAGE_SVG, map);
    insert_mime!("tif", IMAGE_TIFF, map);
    insert_mime!("tiff", IMAGE_TIFF, map);
    insert_mime!("bmp", IMAGE_BMP, map);

    insert_mime!("js", APPLICATION_JAVASCRIPT, map);
    insert_mime!("wasm", APPLICATION_WASM, map);
    insert_mime!("json", APPLICATION_JSON, map);
    insert_mime!("jar", APPLICATION_JAVA_ARCHIVE, map);
    insert_mime!("war", APPLICATION_JAVA_ARCHIVE, map);
    insert_mime!("ear", APPLICATION_JAVA_ARCHIVE, map);
    insert_mime!("m3u8", APPLICATION_APPLE_MPEGURL, map);
    insert_mime!("bin", APPLICATION_OCTET_STREAM, map);
    insert_mime!("exe", APPLICATION_OCTET_STREAM, map);
    insert_mime!("dll", APPLICATION_OCTET_STREAM, map);
    insert_mime!("deb", APPLICATION_OCTET_STREAM, map);
    insert_mime!("dmg", APPLICATION_OCTET_STREAM, map);
    insert_mime!("iso", APPLICATION_OCTET_STREAM, map);
    insert_mime!("img", APPLICATION_OCTET_STREAM, map);
    insert_mime!("msi", APPLICATION_OCTET_STREAM, map);
    insert_mime!("msp", APPLICATION_OCTET_STREAM, map);
    insert_mime!("msm", APPLICATION_OCTET_STREAM, map);

    insert_mime!("woff", FONT_WOFF, map);
    insert_mime!("woff2", FONT_WOFF2, map);

    insert_mime!("ts", VIDEO_MP2T, map);
    insert_mime!("3gpp", VIDEO_3GPP, map);
    insert_mime!("3gp", VIDEO_3GPP, map);
    insert_mime!("mp4", VIDEO_MP4, map);
    insert_mime!("mpeg", VIDEO_MPEG, map);
    insert_mime!("mpg", VIDEO_MPEG, map);
    insert_mime!("mov", VIDEO_QUICKTIME, map);
    insert_mime!("webm", VIDEO_WEBM, map);

    insert_mime!("flv", VIDEO_X_FLV, map);
    insert_mime!("m4v", VIDEO_X_M4V, map);
    insert_mime!("mng", VIDEO_X_MNG, map);
    insert_mime!("asx", VIDEO_X_MS_ASF, map);
    insert_mime!("asf", VIDEO_X_MS_ASF, map);
    insert_mime!("wmv", VIDEO_X_MS_WMV, map);
    insert_mime!("avi", VIDEO_X_MSVIDEO, map);
}
