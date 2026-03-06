//! 服务工具模块
//!
//! 提供服务器运行所需的工具函数。
//!
//! # 功能
//!
//! - 主机端口解析：从 host 字符串中解析端口号
//! - 支持 IPv4、IPv6 和域名格式
//! - 支持默认端口（HTTP: 80, HTTPS: 443）

use tracing::debug;

/// 从 host 字符串中解析端口号
///
/// 支持多种格式的 host 字符串，包括 IPv4、IPv6 和域名。
/// 如果未指定端口，则根据协议返回默认端口。
///
/// # 参数
///
/// * `host` - 主机字符串，格式可以是：
///   - IPv4: "127.0.0.1:8080" 或 "127.0.0.1"
///   - IPv6: "\[::1\]:8080" 或 "\[::1\]"
///   - 域名: "example.com:8080" 或 "example.com"
/// * `scheme` - 协议类型（"http" 或 "https"）
///
/// # 返回值
///
/// 返回解析出的端口号（`Some(port)`）或 `None`（解析失败）。
///
/// # 示例
///
/// ```
/// use candy::utils::parse_port_from_host;
///
/// // IPv4 带端口
/// assert_eq!(parse_port_from_host("127.0.0.1:8080", "http"), Some(8080));
///
/// // IPv4 不带端口（使用默认端口）
/// assert_eq!(parse_port_from_host("127.0.0.1", "http"), Some(80));
/// assert_eq!(parse_port_from_host("127.0.0.1", "https"), Some(443));
///
/// // IPv6 带端口
/// assert_eq!(parse_port_from_host("[::1]:8080", "http"), Some(8080));
///
/// // 域名带端口
/// assert_eq!(parse_port_from_host("example.com:8080", "http"), Some(8080));
///
/// // 域名不带端口
/// assert_eq!(parse_port_from_host("example.com", "http"), Some(80));
///
/// // 不支持的协议
/// assert_eq!(parse_port_from_host("example.com", "ftp"), None);
///
/// // 空字符串
/// assert_eq!(parse_port_from_host("", "http"), None);
/// ```
///
/// # 错误处理
///
/// 以下情况会返回 `None`：
/// - host 字符串为空
/// - 端口号格式错误（非数字）
/// - 端口号超出有效范围（0-65535）
/// - 不支持的协议类型
pub fn parse_port_from_host(host: &str, scheme: &str) -> Option<u16> {
    if host.is_empty() {
        return None;
    }

    // 处理 IPv6 地址，如 [::1]:3000
    if host.starts_with('[') && host.contains(']') {
        // 找到 ]: 之后的部分作为端口
        let port_start = host.find("]:")? + 2;
        let port_str = &host[port_start..];
        return port_str.parse::<u16>().ok();
    }

    let host_parts = host.split(':').collect::<Vec<&str>>();
    let port = if host_parts.len() == 1 {
        match scheme {
            "http" => 80,
            "https" => 443,
            _ => {
                debug!("scheme not support");
                return None;
            }
        }
    } else {
        // 处理 IPv4 或域名带端口的情况
        host_parts.get(1)?.parse::<u16>().ok()?
    };

    Some(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_port_from_host_with_port() {
        // 测试包含端口的主机字符串
        assert_eq!(parse_port_from_host("localhost:8080", "http"), Some(8080));
        assert_eq!(parse_port_from_host("127.0.0.1:9090", "https"), Some(9090));
        assert_eq!(parse_port_from_host("[::1]:3000", "http"), Some(3000));
    }

    #[test]
    fn test_parse_port_from_host_without_port() {
        // 测试不包含端口的主机字符串（使用默认端口）
        assert_eq!(parse_port_from_host("localhost", "http"), Some(80));
        assert_eq!(parse_port_from_host("example.com", "https"), Some(443));
        assert_eq!(parse_port_from_host("192.168.1.1", "http"), Some(80));
    }

    #[test]
    fn test_parse_port_from_host_invalid_scheme() {
        // 测试不支持的协议
        assert_eq!(parse_port_from_host("localhost", "ftp"), None);
        // 当有明确指定的端口时，即使 scheme 无效，也会返回端口
        assert_eq!(parse_port_from_host("example.com:8080", "ws"), Some(8080));
    }

    #[test]
    fn test_parse_port_from_host_invalid_port() {
        // 测试无效的端口号
        assert_eq!(parse_port_from_host("localhost:abc", "http"), None);
        assert_eq!(parse_port_from_host("example.com:port", "https"), None);
        assert_eq!(parse_port_from_host("localhost:65536", "http"), None); // 超出 u16 范围
    }

    #[test]
    fn test_parse_port_from_host_empty_string() {
        // 测试空字符串
        assert_eq!(parse_port_from_host("", "http"), None);
        assert_eq!(parse_port_from_host(":", "https"), None);
    }
}
