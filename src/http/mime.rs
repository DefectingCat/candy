// https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
#![allow(dead_code)]

macro_rules! mime {
    ($a:ident, $b:literal) => {
        pub const $a: &str = $b;
    };
}

mime!(TEXT_PLAIN, "text/plain");
mime!(TEXT_PLAIN_UTF_8, "text/plain; charset=utf-8");
mime!(TEXT_HTML, "text/html");
mime!(TEXT_HTML_UTF_8, "text/html; charset=utf-8");
mime!(TEXT_CSS, "text/css");
mime!(TEXT_CSS_UTF_8, "text/css; charset=utf-8");
mime!(TEXT_JAVASCRIPT, "text/javascript");
mime!(TEXT_XML, "text/xml");
mime!(TEXT_EVENT_STREAM, "text/event-stream");
mime!(TEXT_CSV, "text/csv");
mime!(TEXT_CSV_UTF_8, "text/csv; charset=utf-8");
mime!(TEXT_TAB_SEPARATED_VALUES, "text/tab-separated-values");
mime!(
    TEXT_TAB_SEPARATED_VALUES_UTF_8,
    "text/tab-separated-values; charset=utf-8"
);
mime!(TEXT_VCARD, "text/vcard");

mime!(IMAGE_JPEG, "image/jpeg");
mime!(IMAGE_GIF, "image/gif");
mime!(IMAGE_PNG, "image/png");
mime!(IMAGE_ICON, "image/x-icon");
mime!(IMAGE_JNG, "image/x-jng");
mime!(IMAGE_WBMP, "image/vnd.wap.wbmp ");
mime!(IMAGE_BMP, "image/bmp");
mime!(IMAGE_SVG, "image/svg+xml");
mime!(IMAGE_AVIF, "image/avif");
mime!(IMAGE_TIFF, "image/tiff");
mime!(IMAGE_WEBP, "image/webp");

mime!(VIDEO_3GPP, "video/3gpp");
mime!(VIDEO_MP2T, "video/mp2t");
mime!(VIDEO_MP4, "video/mp4");
mime!(VIDEO_MPEG, "video/mpeg");
mime!(VIDEO_QUICKTIME, "video/quicktime");
mime!(VIDEO_WEBM, "video/webm");
mime!(VIDEO_X_FLV, "video/x-flv");
mime!(VIDEO_X_M4V, "video/x-m4v");
mime!(VIDEO_X_MNG, "video/x-mng");
mime!(VIDEO_X_MS_ASF, "video/x-ms-asf");
mime!(VIDEO_X_MS_WMV, "video/x-ms-wmv");
mime!(VIDEO_X_MSVIDEO, "video/x-msvideo");

mime!(FONT_WOFF, "font/woff");
mime!(FONT_WOFF2, "font/woff2");

mime!(APPLICATION_JSON, "application/json");
mime!(APPLICATION_JAVASCRIPT, "application/javascript");
mime!(APPLICATION_WASM, "application/wasm");
mime!(
    APPLICATION_JAVASCRIPT_UTF_8,
    "application/javascript; charset=utf-8"
);
mime!(
    APPLICATION_WWW_FORM_URLENCODED,
    "application/x-www-form-urlencoded"
);
mime!(APPLICATION_OCTET_STREAM, "application/octet-stream");
mime!(APPLICATION_MSGPACK, "application/msgpack");
mime!(APPLICATION_PDF, "application/pdf");
mime!(APPLICATION_DNS, "application/dns-message");
mime!(APPLICATION_JAVA_ARCHIVE, "application/java-archive");
mime!(APPLICATION_APPLE_MPEGURL, "application/vnd.apple.mpegurl");

mime!(STAR_STAR, "*/*");
mime!(TEXT_STAR, "text/*");
mime!(IMAGE_STAR, "image/*");
mime!(VIDEO_STAR, "video/*");
mime!(AUDIO_STAR, "audio/*");
