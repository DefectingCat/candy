#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_upstream_config() {
        // 创建包含 upstream 配置的临时 TOML 文件
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[upstream]]
            name = "test_backend"
            server = [
                {{ server = "192.168.1.100:8080" }},
                {{ server = "192.168.1.101:8080" }}
            ]

            [[host]]
            ip = "127.0.0.1"
            port = 8080
            ssl = false

            [[host.route]]
            location = "/api"
            upstream = "test_backend"
            proxy_timeout = 30
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(path).unwrap();

        // 验证 upstream 配置
        assert!(settings.upstream.is_some());
        let upstreams = settings.upstream.as_ref().unwrap();
        assert_eq!(upstreams.len(), 1);

        let test_backend = &upstreams[0];
        assert_eq!(test_backend.name, "test_backend");
        assert_eq!(test_backend.server.len(), 2);
        assert_eq!(test_backend.server[0].server, "192.168.1.100:8080");
        assert_eq!(test_backend.server[1].server, "192.168.1.101:8080");

        // 验证路由配置
        assert_eq!(settings.host.len(), 1);
        let route = &settings.host[0].route[0];
        assert_eq!(route.location, "/api");
        assert_eq!(route.upstream, Some("test_backend".to_string()));
        assert_eq!(route.proxy_timeout, 30);
    }

    #[test]
    fn test_invalid_upstream_config() {
        // 测试无效的 upstream 配置（空服务器列表）
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[upstream]]
            name = "invalid_backend"
            server = []

            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            upstream = "invalid_backend"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_upstream_config() {
        // 测试引用不存在的 upstream
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
            [[host]]
            ip = "127.0.0.1"
            port = 8080

            [[host.route]]
            location = "/"
            upstream = "nonexistent_backend"
            "#,
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let result = Settings::new(path);
        assert!(result.is_err());
    }
}
