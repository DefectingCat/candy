use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;

/// 支持的压缩算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    Gzip,
    Deflate,
    None,
}

/// 解析 Accept-Encoding 头
/// 返回支持的压缩类型，优先选择 gzip
pub fn parse_accept_encoding(header: &[u8]) -> CompressionType {
    let header_str = String::from_utf8_lossy(header);
    let header_lower = header_str.to_lowercase();

    // 解析编码和 q 值
    let mut gzip_q: f32 = 0.0;
    let mut deflate_q: f32 = 0.0;
    let mut wildcard_q: f32 = 0.0;

    for part in header_lower.split(',') {
        let part = part.trim();
        let (encoding, q) = parse_encoding_part(part);

        match encoding {
            "gzip" | "x-gzip"
                if q > gzip_q => {
                    gzip_q = q;
                }
            "deflate"
                if q > deflate_q => {
                    deflate_q = q;
                }
            "*"
                if q > wildcard_q => {
                    wildcard_q = q;
                }
            _ => {}
        }
    }

    // 优先选择 gzip（相同 q 值时 gzip 优先）
    if gzip_q > 0.0 && gzip_q >= deflate_q {
        return CompressionType::Gzip;
    }
    if deflate_q > 0.0 {
        return CompressionType::Deflate;
    }
    if wildcard_q > 0.0 {
        return CompressionType::Gzip;
    }

    CompressionType::None
}

/// 解析单个编码部分，返回 (编码名, q值)
fn parse_encoding_part(part: &str) -> (&str, f32) {
    let parts: Vec<&str> = part.split(';').collect();
    let encoding = parts[0].trim();

    // 默认 q=1.0
    let mut q: f32 = 1.0;

    for p in &parts[1..] {
        let p = p.trim();
        if p.starts_with("q=")
            && let Ok(val) = p[2..].parse::<f32>() {
                q = val;
            }
    }

    (encoding, q)
}

/// 使用 gzip 压缩数据
pub fn gzip_compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder.finish()
}

/// 检查 MIME 类型是否适合压缩
/// 文本类型适合压缩，已压缩的二进制类型不适合
pub fn should_compress(mime_type: &str) -> bool {
    let mime_lower = mime_type.to_lowercase();

    // 文本类型适合压缩
    if mime_lower.starts_with("text/")
        || mime_lower.contains("json")
        || mime_lower.contains("xml")
        || mime_lower.contains("javascript")
        || mime_lower == "application/wasm"
    {
        return true;
    }

    // 已压缩的类型不适合再压缩
    let already_compressed = [
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "video/",
        "audio/",
        "application/pdf",
        "application/zip",
        "application/gzip",
        "application/x-gzip",
    ];

    for &compressed in &already_compressed {
        if mime_lower.starts_with(compressed) {
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accept_encoding_gzip() {
        let result = parse_accept_encoding(b"gzip");
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn test_parse_accept_encoding_deflate() {
        let result = parse_accept_encoding(b"deflate");
        assert_eq!(result, CompressionType::Deflate);
    }

    #[test]
    fn test_parse_accept_encoding_multiple() {
        let result = parse_accept_encoding(b"gzip, deflate");
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn test_parse_accept_encoding_with_q() {
        // deflate q=1.0 (默认), gzip q=1.0 (默认), * q=0.5
        // gzip 和 deflate 都是 q=1.0，gzip 优先
        let result = parse_accept_encoding(b"deflate, gzip;q=1.0, *;q=0.5");
        assert_eq!(result, CompressionType::Gzip);
    }

    #[test]
    fn test_parse_accept_encoding_deflate_higher_q() {
        // deflate q=1.0, gzip q=0.5
        let result = parse_accept_encoding(b"gzip;q=0.5, deflate");
        assert_eq!(result, CompressionType::Deflate);
    }

    #[test]
    fn test_parse_accept_encoding_empty() {
        let result = parse_accept_encoding(b"");
        assert_eq!(result, CompressionType::None);
    }

    #[test]
    fn test_parse_accept_encoding_identity() {
        let result = parse_accept_encoding(b"identity");
        assert_eq!(result, CompressionType::None);
    }

    #[test]
    fn test_gzip_compress() {
        // 使用较长的重复数据确保压缩效果
        let data = b"Hello, World! This is a test string for compression. Hello, World! This is a test string for compression. Hello, World! This is a test string for compression.";
        let compressed = gzip_compress(data).unwrap();

        // 压缩后应该比原始数据小（对于这种重复性文本）
        assert!(compressed.len() < data.len());

        // 验证 gzip 头部
        assert_eq!(&compressed[0..2], &[0x1f, 0x8b]); // gzip magic number
    }

    #[test]
    fn test_gzip_compress_short() {
        // 短数据压缩后可能更大（gzip 头部开销）
        let data = b"Hello";
        let compressed = gzip_compress(data).unwrap();

        // 验证 gzip 头部
        assert_eq!(&compressed[0..2], &[0x1f, 0x8b]); // gzip magic number
    }

    #[test]
    fn test_should_compress_text() {
        assert!(should_compress("text/html"));
        assert!(should_compress("text/css"));
        assert!(should_compress("text/javascript"));
        assert!(should_compress("application/json"));
        assert!(should_compress("application/javascript"));
        assert!(should_compress("application/xml"));
    }

    #[test]
    fn test_should_compress_not_for_images() {
        assert!(!should_compress("image/jpeg"));
        assert!(!should_compress("image/png"));
        assert!(!should_compress("image/gif"));
        assert!(!should_compress("image/webp"));
    }

    #[test]
    fn test_should_compress_not_for_video() {
        assert!(!should_compress("video/mp4"));
        assert!(!should_compress("audio/mp3"));
    }

    #[test]
    fn test_should_compress_not_for_already_compressed() {
        assert!(!should_compress("application/pdf"));
        assert!(!should_compress("application/zip"));
        assert!(!should_compress("application/gzip"));
    }
}
