use std::path::{Path, PathBuf};

/// 路径规范化结果
#[derive(Debug)]
#[allow(dead_code)]
pub enum ResolveResult {
    /// 找到文件
    File(PathBuf),
    /// 找到目录，需要查找 index.html
    #[allow(dead_code)]
    Directory(PathBuf),
    /// 路径不存在
    NotFound,
    /// 路径穿越攻击
    Forbidden,
}

/// 解析 URL 路径到文件系统路径
///
/// # 安全性
/// 防止目录穿越攻击（如 /../../../etc/passwd）
pub fn resolve_path(base: &Path, url: &str) -> ResolveResult {
    // 规范化 URL 路径
    let normalized = normalize_url_path(url);

    // 检查是否包含可疑组件
    if contains_traversal(&normalized) {
        return ResolveResult::Forbidden;
    }

    // 拼接基础路径
    let full_path = base.join(&normalized);

    // 安全检查：确保结果路径在 base 目录内
    match full_path.canonicalize() {
        Ok(canonical) => {
            // 检查是否在 base 目录内
            match base.canonicalize() {
                Ok(base_canonical) => {
                    if !canonical.starts_with(&base_canonical) {
                        return ResolveResult::Forbidden;
                    }
                }
                Err(_) => {
                    // base 目录不存在，无法验证
                    return ResolveResult::NotFound;
                }
            }

            if canonical.is_dir() {
                // 尝试查找 index.html
                let index = canonical.join("index.html");
                if index.exists() {
                    ResolveResult::File(index)
                } else {
                    ResolveResult::Directory(canonical)
                }
            } else if canonical.exists() {
                ResolveResult::File(canonical)
            } else {
                ResolveResult::NotFound
            }
        }
        Err(_) => {
            // 路径不存在
            ResolveResult::NotFound
        }
    }
}

/// 规范化 URL 路径
fn normalize_url_path(url: &str) -> String {
    // 移除查询字符串
    let path = url.split('?').next().unwrap_or(url);

    // 解码 URL 编码（简单实现，只处理 %XX）
    let decoded = url_decode(path);

    // 移除开头的 /
    decoded.trim_start_matches('/').to_string()
}

/// 简单的 URL 解码
fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// 检查路径是否包含目录穿越
fn contains_traversal(path: &str) -> bool {
    let components: Vec<&str> = path.split('/').collect();

    for component in &components {
        if *component == ".." {
            return true;
        }
    }

    false
}

/// 获取文件的 MIME 类型
pub fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolve_valid_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.html");
        fs::write(&file_path, "test").unwrap();

        let result = resolve_path(temp_dir.path(), "/test.html");
        match result {
            ResolveResult::File(p) => assert_eq!(p, file_path),
            _ => panic!("Expected File"),
        }
    }

    #[test]
    fn test_resolve_traversal_attack() {
        let temp_dir = tempfile::tempdir().unwrap();

        let result = resolve_path(temp_dir.path(), "/../../../etc/passwd");
        match result {
            ResolveResult::Forbidden => {}
            _ => panic!("Expected Forbidden"),
        }
    }

    #[test]
    fn test_resolve_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();

        let result = resolve_path(temp_dir.path(), "/nonexistent.html");
        match result {
            ResolveResult::NotFound => {}
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn test_resolve_index_html() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();
        let index_path = dir_path.join("index.html");
        fs::write(&index_path, "index").unwrap();

        let result = resolve_path(temp_dir.path(), "/subdir/");
        match result {
            ResolveResult::File(p) => assert_eq!(p, index_path),
            _ => panic!("Expected File with index.html"),
        }
    }

    #[test]
    fn test_get_mime_type() {
        assert_eq!(
            get_mime_type(Path::new("test.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            get_mime_type(Path::new("test.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            get_mime_type(Path::new("test.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(get_mime_type(Path::new("test.png")), "image/png");
        assert_eq!(
            get_mime_type(Path::new("test.unknown")),
            "application/octet-stream"
        );
    }
}
