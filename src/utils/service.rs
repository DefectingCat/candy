use tracing::debug;

/// Parse port from host
/// if host is localhost:8080
/// return 8080
/// if host is localhost
/// return 80
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
