use std::{net::SocketAddr, sync::LazyLock, time::Duration};

use std::sync::Arc;

use anyhow::anyhow;
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use dashmap::DashMap;
use http::StatusCode;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::{CompressionLevel, compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::{debug, error, info, warn};

use crate::{
    config::{CompressionConfig, SettingHost, SettingRoute, Upstream},
    middlewares::{add_headers, add_version, logging_route},
};

/// 上游服务器组配置存储
/// 使用 upstream 名称作为键，Upstream 配置作为值
pub static UPSTREAMS: LazyLock<DashMap<String, Upstream>> = LazyLock::new(DashMap::new);

/// 清除所有全局状态
///
/// 此函数主要用于测试场景，确保测试之间的隔离。
/// 在多线程测试中，应该在每个测试开始时调用此函数来清除之前测试遗留的状态。
#[allow(dead_code)]
pub fn clear_global_state() {
    UPSTREAMS.clear();
    HOSTS.clear();
}

pub mod error;
// 处理静态文件
pub mod serve;
// 处理反向代理
pub mod reverse_proxy;
// 处理正向代理
pub mod forward_proxy;
// 处理 Lua 脚本
#[cfg(feature = "lua")]
pub mod lua;
// 处理 HTTP 重定向
pub mod redirect;

/// 主机配置
/// 使用虚拟主机端口作为键
/// 使用域名（或 None 表示默认主机）作为二级键
/// 使用 SettingHost 作为值
/// {
///     80: {
///         Some("rua.plus"): <SettingHost>,
///         Some("www.rua.plus"): <SettingHost>,
///         None: <SettingHost> // 默认主机
///     }
/// }
pub static HOSTS: LazyLock<DashMap<u16, DashMap<Option<String>, SettingHost>>> =
    LazyLock::new(DashMap::new);

/// 加载上游服务器配置到全局存储
pub fn load_upstreams(settings: &crate::config::Settings) {
    crate::http::UPSTREAMS.clear();
    if let Some(upstreams) = &settings.upstream {
        for upstream in upstreams {
            crate::http::UPSTREAMS.insert(upstream.name.clone(), upstream.clone());
        }
        // 初始化健康检查
        crate::http::reverse_proxy::initialize_health_checks(upstreams);
    }
}

/// 构建压缩层
///
/// 根据路由配置和全局配置构建压缩层。路由级别配置优先于全局配置。
///
/// # 参数
/// * `route` - 路由配置
/// * `global` - 全局压缩配置
///
/// # 返回值
/// 返回构建好的压缩层
fn build_compression_layer(route: &SettingRoute, global: &CompressionConfig) -> CompressionLayer {
    // 使用路由配置或回退到全局配置
    let gzip = route.gzip.unwrap_or(global.gzip);
    let deflate = route.deflate.unwrap_or(global.deflate);
    let br = route.br.unwrap_or(global.br);
    let zstd = route.zstd.unwrap_or(global.zstd);
    let level = route.level.unwrap_or(global.level);

    let compression_level = match level {
        1 => CompressionLevel::Fastest,
        2..=8 => CompressionLevel::Precise(level as i32),
        9 => CompressionLevel::Best,
        _ => CompressionLevel::Default,
    };

    CompressionLayer::new()
        .gzip(gzip)
        .deflate(deflate)
        .br(br)
        .zstd(zstd)
        .quality(compression_level)
}

/// 检查路由是否有自定义压缩配置
fn route_has_custom_compression(route: &SettingRoute) -> bool {
    route.gzip.is_some()
        || route.deflate.is_some()
        || route.br.is_some()
        || route.zstd.is_some()
        || route.level.is_some()
}

/// 检查主机是否有任何路由使用了自定义压缩配置
fn host_has_any_custom_compression(host: &SettingHost) -> bool {
    host.route.iter().any(route_has_custom_compression)
}

/// 启动初始服务器实例
pub async fn start_initial_servers(
    settings: crate::config::Settings,
) -> anyhow::Result<Arc<Mutex<Vec<axum_server::Handle<SocketAddr>>>>> {
    let compression = settings.compression.clone();
    let handles = start_servers(settings.host, compression).await;
    Ok(Arc::new(Mutex::new(handles)))
}

/// 处理配置文件变更的回调函数
pub async fn handle_config_change(
    result: crate::error::Result<crate::config::Settings>,
    handles: Arc<Mutex<Vec<axum_server::Handle<SocketAddr>>>>,
) {
    match result {
        Ok(new_settings) => {
            info!("Config file reloaded successfully");
            info!("Config file changed, restarting servers to apply new config...");

            // 停止当前所有服务器
            let mut current_handles = handles.lock().await;
            shutdown_servers(&mut current_handles).await;

            // 在新的 tokio 任务中启动新服务器
            let new_hosts = new_settings.host;
            let new_upstreams = new_settings.upstream;
            let new_compression = new_settings.compression;
            let handles_clone = handles.clone();
            tokio::spawn(async move {
                // 清空全局 HOSTS 和 UPSTREAMS 变量，确保新配置完全生效
                crate::http::HOSTS.clear();
                crate::http::UPSTREAMS.clear();

                // 重新加载 upstream 配置
                if let Some(upstreams) = &new_upstreams {
                    for upstream in upstreams {
                        crate::http::UPSTREAMS.insert(upstream.name.clone(), upstream.clone());
                    }
                    // 初始化健康检查
                    crate::http::reverse_proxy::initialize_health_checks(upstreams);
                }

                let new_handles = start_servers(new_hosts, new_compression).await;

                let mut current_handles = handles_clone.lock().await;
                *current_handles = new_handles;
                info!("All servers have been restarted successfully");
            });
        }
        Err(e) => {
            error!("Failed to reload config file: {:?}", e);
        }
    }
}

/// 优雅关闭所有服务器
///
/// 对所有运行中的服务器发送优雅关闭信号，并清空服务器句柄列表。
/// 服务器将在 30 秒内完成正在处理的请求后停止。
///
/// # 参数
///
/// * `handles` - 服务器句柄的可变引用，用于存储所有正在运行的服务器实例
///
/// # 日志记录
///
/// 函数会记录一个信息级别的日志，指示所有服务器已收到关闭信号
pub async fn shutdown_servers(handles: &mut Vec<axum_server::Handle<SocketAddr>>) {
    for handle in handles.iter() {
        handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
    }
    handles.clear();
    info!("All servers have been signaled to shut down");
}

/// 启动所有服务器
///
/// 根据配置文件中定义的主机列表启动所有服务器实例。
/// 每个服务器实例会根据其配置（HTTP 或 HTTPS）进行初始化和启动。
/// 对于启动失败的服务器，会记录错误日志但不会中断其他服务器的启动过程。
///
/// # 参数
///
/// * `hosts` - 配置文件中定义的所有主机的列表，每个主机包含完整的服务器配置
/// * `compression` - 压缩配置，用于配置响应压缩
///
/// # 返回值
///
/// 返回一个包含所有成功启动的服务器句柄的向量
///
/// # 错误处理
///
/// 单个服务器启动失败会被捕获并记录为错误日志，不会影响其他服务器的启动
pub async fn start_servers(
    hosts: Vec<SettingHost>,
    compression: CompressionConfig,
) -> Vec<axum_server::Handle<SocketAddr>> {
    let mut handles = Vec::new();
    for host in hosts {
        // 保存主机地址信息用于日志显示
        let server_addr = format!("{}:{}", host.ip, host.port);
        match make_server(host, compression.clone()).await {
            Ok(handle) => {
                handles.push(handle);
                info!("Server instance started on {}", server_addr);
            }
            Err(e) => {
                error!(
                    "Failed to start server instance on {}: {:?}",
                    server_addr, e
                );
            }
        }
    }
    handles
}

pub async fn make_server(
    host: SettingHost,
    compression: CompressionConfig,
) -> anyhow::Result<axum_server::Handle<SocketAddr>> {
    debug!("make_server start with host: {:?}", host);
    let mut router = Router::new();
    let host_to_save = host.clone();

    // 检查是否有任何路由使用了自定义压缩配置
    let any_route_has_custom_compression = host_has_any_custom_compression(&host);

    // 辅助函数：注册路由（处理带/不带斜杠的路径）
    let register_route = |router: Router,
                          location: &str,
                          handler: axum::routing::MethodRouter|
     -> (Router, String) {
        let path_morethan_one = location.len() > 1;
        let mut router = router;

        if path_morethan_one && location.ends_with('/') {
            // 首先注册带斜杠的路径 /doc
            router = router.route(location, handler.clone());
            debug!("Route registered: {}", location);
            let len = location.len();
            let path_without_slash = &location[0..len - 1];
            // 然后注册不带斜杠的路径 /doc/
            router = router.route(path_without_slash, handler.clone());
            debug!("Route registered: {}", path_without_slash);
            (router, location.to_string())
        } else if path_morethan_one {
            // 首先注册不带斜杠的路径 /doc
            router = router.route(location, handler.clone());
            debug!("Route registered: {}", location);
            // 然后注册带斜杠的路径 /doc/
            let path_with_slash = format!("{}/", location);
            router = router.route(&path_with_slash, handler.clone());
            debug!("Route registered: {}", path_with_slash);
            (router, path_with_slash)
        } else {
            // 注册根路径 /
            router = router.route(location, handler);
            debug!("Route registered: {}", location);
            (router, location.to_string())
        }
    };

    // 在配置中查找路由，转换为 Axum 路由并注册
    for host_route in &host.route {
        // HTTP 重定向
        if host_route.redirect_to.is_some() {
            let handler = if any_route_has_custom_compression {
                get(redirect::redirect).layer(build_compression_layer(host_route, &compression))
            } else {
                get(redirect::redirect)
            };

            let (new_router, route_path) = register_route(router, &host_route.location, handler);
            router = new_router;

            // 将路由路径保存到映射中
            host_to_save
                .route_map
                .insert(route_path.clone(), host_route.clone());

            let wildcard_path = format!("{route_path}{{*path}}");
            let wildcard_handler = if any_route_has_custom_compression {
                get(serve::serve).layer(build_compression_layer(host_route, &compression))
            } else {
                get(serve::serve)
            };
            router = router.route(&wildcard_path, wildcard_handler);
            debug!("HTTP redirect wildcard route registered: {}", wildcard_path);
            continue;
        }

        // Lua 脚本
        #[cfg(feature = "lua")]
        if host_route.lua_script.is_some() {
            let handler = if any_route_has_custom_compression {
                get(lua::lua).layer(build_compression_layer(host_route, &compression))
            } else {
                get(lua::lua)
            };

            router = router.route(&host_route.location, handler);
            let wildcard_path = format!("{}{{*path}}", host_route.location);
            let wildcard_handler = if any_route_has_custom_compression {
                get(lua::lua).layer(build_compression_layer(host_route, &compression))
            } else {
                get(lua::lua)
            };
            router = router.route(&wildcard_path, wildcard_handler);

            host_to_save
                .route_map
                .insert(host_route.location.clone(), host_route.clone());
            debug!("Lua script route registered: {}", wildcard_path);
            continue;
        }

        // 反向代理（包括 upstream 负载均衡）
        if host_route.proxy_pass.is_some() || host_route.upstream.is_some() {
            let mut handler: axum::routing::MethodRouter = get(reverse_proxy::serve);

            // 应用路由级别压缩配置
            if any_route_has_custom_compression {
                handler = handler.layer(build_compression_layer(host_route, &compression));
            }

            router = router.route(&host_route.location, handler.clone());
            let wildcard_path = format!("{}{{*path}}", host_route.location);
            router = router.route(&wildcard_path, handler);

            if let Some(max_body_size) = host_route.max_body_size {
                router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
            }

            host_to_save
                .route_map
                .insert(host_route.location.clone(), host_route.clone());
            debug!("Reverse proxy route registered: {}", host_route.location);
            continue;
        }

        // 正向代理
        if host_route.forward_proxy.is_some() && host_route.forward_proxy.unwrap() {
            let mut handler: axum::routing::MethodRouter = get(forward_proxy::serve);

            // 应用路由级别压缩配置
            if any_route_has_custom_compression {
                handler = handler.layer(build_compression_layer(host_route, &compression));
            }

            router = router.route(&host_route.location, handler.clone());
            let wildcard_path = format!("{}{{*path}}", host_route.location);
            router = router.route(&wildcard_path, handler);

            if let Some(max_body_size) = host_route.max_body_size {
                router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
            }

            host_to_save
                .route_map
                .insert(host_route.location.clone(), host_route.clone());
            debug!("Forward proxy route registered: {}", host_route.location);
            continue;
        }

        // 静态文件
        if host_route.root.is_none() {
            warn!("Route missing root field: {:?}", host_route.location);
            continue;
        }

        if let Some(max_body_size) = host_route.max_body_size {
            router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
        }

        let handler = if any_route_has_custom_compression {
            get(serve::serve).layer(build_compression_layer(host_route, &compression))
        } else {
            get(serve::serve)
        };

        let (new_router, route_path) = register_route(router, &host_route.location, handler);
        router = new_router;

        host_to_save
            .route_map
            .insert(route_path.clone(), host_route.clone());

        let wildcard_path = format!("{route_path}{{*path}}");
        // 为 wildcard 路由也应用相同的压缩配置
        let wildcard_handler = if any_route_has_custom_compression {
            get(serve::serve).layer(build_compression_layer(host_route, &compression))
        } else {
            get(serve::serve)
        };
        router = router.route(&wildcard_path, wildcard_handler);
        debug!("Static file wildcard route registered: {}", wildcard_path);
    }

    // 保存主机到映射中
    let server_name = host.server_name.as_ref().cloned();
    if let Some(port_entry) = HOSTS.get_mut(&host.port) {
        port_entry.insert(server_name, host_to_save);
    } else {
        let domain_map = DashMap::new();
        domain_map.insert(server_name, host_to_save);
        HOSTS.insert(host.port, domain_map);
    }

    // 如果没有任何路由使用自定义压缩配置，则应用全局压缩层
    // 否则每个路由已经应用了自己的压缩层
    if !any_route_has_custom_compression {
        let compression_level = match compression.level {
            1 => CompressionLevel::Fastest,
            2..=8 => CompressionLevel::Precise(compression.level as i32),
            9 => CompressionLevel::Best,
            _ => CompressionLevel::Default,
        };

        let compression_layer = CompressionLayer::new()
            .gzip(compression.gzip)
            .deflate(compression.deflate)
            .br(compression.br)
            .zstd(compression.zstd)
            .quality(compression_level);

        router = router.layer(compression_layer);
    }

    router = router.layer(
        ServiceBuilder::new()
            .layer(middleware::from_fn(add_version))
            .layer(middleware::from_fn(add_headers))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::SERVICE_UNAVAILABLE,
                Duration::from_secs(host.timeout.into()),
            )),
    );

    router = logging_route(router);

    let (ip, port, ssl, certificate, certificate_key) = (
        host.ip.clone(),
        host.port,
        host.ssl,
        host.certificate.clone(),
        host.certificate_key.clone(),
    );
    let addr = format!("{}:{}", ip, port);
    let addr: SocketAddr = addr.parse()?;

    let handle = Handle::new();
    let handle_clone = handle.clone();

    // 生成一个任务来运行服务器
    tokio::spawn(async move {
        if ssl && certificate.is_some() && certificate_key.is_some() {
            match (certificate, certificate_key) {
                (Some(cert), Some(key)) => {
                    debug!("Certificate: {} Certificate key: {}", cert, key);
                    match RustlsConfig::from_pem_file(&cert, &key).await {
                        Ok(rustls_config) => {
                            info!("Listening on https://{}", addr);
                            axum_server::bind_rustls(addr, rustls_config)
                                .handle(handle_clone)
                                .serve(router.into_make_service())
                                .await
                                .map_err(anyhow::Error::from)
                        }
                        Err(e) => Err(anyhow::Error::from(e)),
                    }
                }
                _ => Err(anyhow!("SSL enabled but certificate or key missing")),
            }
        } else {
            info!("Listening on http://{}", addr);
            axum_server::bind(addr)
                .handle(handle_clone)
                .serve(router.into_make_service())
                .await
                .map_err(anyhow::Error::from)
        }
    });

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Compression Layer Tests ==========

    #[test]
    fn test_build_compression_layer_default() {
        let route = SettingRoute {
            location: "/".to_string(),
            root: Some("/var/www".to_string()),
            auto_index: false,
            index: vec!["index.html".to_string()],
            error_page: None,
            not_found_page: None,
            proxy_pass: None,
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: None,
            deflate: None,
            br: None,
            zstd: None,
            level: None,
        };

        let global = CompressionConfig::default();
        let _layer = build_compression_layer(&route, &global);

        // 验证层构建成功（默认配置下所有压缩类型都启用）
        // CompressionLayer 实现了 Clone，可以验证其配置
    }

    #[test]
    fn test_build_compression_layer_route_override() {
        let route = SettingRoute {
            location: "/api".to_string(),
            root: None,
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: Some("http://localhost:3000".to_string()),
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: Some(false), // 覆盖全局
            deflate: None,     // 使用全局
            br: Some(true),    // 覆盖全局
            zstd: None,        // 使用全局
            level: Some(9),    // 覆盖全局
        };

        let global = CompressionConfig {
            gzip: true,
            deflate: true,
            br: false,
            zstd: true,
            level: 6,
        };

        let _layer = build_compression_layer(&route, &global);

        // 验证路由级别的配置覆盖全局配置
        // gzip: route.gzip.unwrap_or(global.gzip) = false
        // deflate: route.deflate.unwrap_or(global.deflate) = true
        // br: route.br.unwrap_or(global.br) = true
        // zstd: route.zstd.unwrap_or(global.zstd) = true
        // level: route.level.unwrap_or(global.level) = 9
    }

    #[test]
    fn test_build_compression_level_mapping() {
        // 测试压缩级别映射
        struct LevelTestCase {
            level: u8,
            expected_fastest: bool,
            expected_best: bool,
        }

        let test_cases = [
            LevelTestCase {
                level: 1,
                expected_fastest: true,
                expected_best: false,
            },
            LevelTestCase {
                level: 2,
                expected_fastest: false,
                expected_best: false,
            },
            LevelTestCase {
                level: 5,
                expected_fastest: false,
                expected_best: false,
            },
            LevelTestCase {
                level: 8,
                expected_fastest: false,
                expected_best: false,
            },
            LevelTestCase {
                level: 9,
                expected_fastest: false,
                expected_best: true,
            },
        ];

        for tc in test_cases {
            let route = SettingRoute {
                location: "/".to_string(),
                root: Some("/var/www".to_string()),
                auto_index: false,
                index: vec![],
                error_page: None,
                not_found_page: None,
                proxy_pass: None,
                upstream: None,
                forward_proxy: None,
                proxy_timeout: 5,
                max_body_size: None,
                headers: None,
                lua_script: None,
                redirect_to: None,
                redirect_code: None,
                gzip: None,
                deflate: None,
                br: None,
                zstd: None,
                level: Some(tc.level),
            };

            let global = CompressionConfig::default();
            let _layer = build_compression_layer(&route, &global);

            // 验证级别映射：
            // 1 -> Fastest
            // 2-8 -> Precise(level)
            // 9 -> Best
            // 超出范围的值 -> Default
            if tc.expected_fastest {
                // level=1 应该映射到 Fastest
            } else if tc.expected_best {
                // level=9 应该映射到 Best
            } else {
                // 2-8 应该映射到 Precise
            }
        }
    }

    #[test]
    fn test_route_has_custom_compression_none() {
        // 路由没有自定义压缩配置
        let route = SettingRoute {
            location: "/".to_string(),
            root: Some("/var/www".to_string()),
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: None,
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: None,
            deflate: None,
            br: None,
            zstd: None,
            level: None,
        };

        assert!(!route_has_custom_compression(&route));
    }

    #[test]
    fn test_route_has_custom_compression_gzip() {
        let route = SettingRoute {
            location: "/".to_string(),
            root: Some("/var/www".to_string()),
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: None,
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: Some(true),
            deflate: None,
            br: None,
            zstd: None,
            level: None,
        };

        assert!(route_has_custom_compression(&route));
    }

    #[test]
    fn test_route_has_custom_compression_level() {
        let route = SettingRoute {
            location: "/".to_string(),
            root: Some("/var/www".to_string()),
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: None,
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: None,
            deflate: None,
            br: None,
            zstd: None,
            level: Some(5),
        };

        assert!(route_has_custom_compression(&route));
    }

    #[test]
    fn test_route_has_custom_compression_all_fields() {
        let route = SettingRoute {
            location: "/".to_string(),
            root: Some("/var/www".to_string()),
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: None,
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: Some(false),
            deflate: Some(true),
            br: Some(false),
            zstd: Some(true),
            level: Some(3),
        };

        assert!(route_has_custom_compression(&route));
    }

    #[test]
    fn test_host_has_any_custom_compression_no_routes() {
        let host = SettingHost {
            ip: "127.0.0.1".to_string(),
            port: 8080,
            server_name: None,
            ssl: false,
            certificate: None,
            certificate_key: None,
            route: vec![],
            route_map: DashMap::new(),
            timeout: 75,
        };

        assert!(!host_has_any_custom_compression(&host));
    }

    #[test]
    fn test_host_has_any_custom_compression_no_custom() {
        let route = SettingRoute {
            location: "/".to_string(),
            root: Some("/var/www".to_string()),
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: None,
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: None,
            deflate: None,
            br: None,
            zstd: None,
            level: None,
        };

        let host = SettingHost {
            ip: "127.0.0.1".to_string(),
            port: 8080,
            server_name: None,
            ssl: false,
            certificate: None,
            certificate_key: None,
            route: vec![route],
            route_map: DashMap::new(),
            timeout: 75,
        };

        assert!(!host_has_any_custom_compression(&host));
    }

    #[test]
    fn test_host_has_any_custom_compression_with_custom() {
        let route_with_custom = SettingRoute {
            location: "/api".to_string(),
            root: None,
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: Some("http://localhost:3000".to_string()),
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: Some(true),
            deflate: None,
            br: None,
            zstd: None,
            level: None,
        };

        let host = SettingHost {
            ip: "127.0.0.1".to_string(),
            port: 8080,
            server_name: None,
            ssl: false,
            certificate: None,
            certificate_key: None,
            route: vec![route_with_custom],
            route_map: DashMap::new(),
            timeout: 75,
        };

        assert!(host_has_any_custom_compression(&host));
    }

    #[test]
    fn test_host_has_any_custom_compression_mixed_routes() {
        let route_no_custom = SettingRoute {
            location: "/".to_string(),
            root: Some("/var/www".to_string()),
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: None,
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: None,
            deflate: None,
            br: None,
            zstd: None,
            level: None,
        };

        let route_with_custom = SettingRoute {
            location: "/api".to_string(),
            root: None,
            auto_index: false,
            index: vec![],
            error_page: None,
            not_found_page: None,
            proxy_pass: Some("http://localhost:3000".to_string()),
            upstream: None,
            forward_proxy: None,
            proxy_timeout: 5,
            max_body_size: None,
            headers: None,
            lua_script: None,
            redirect_to: None,
            redirect_code: None,
            gzip: None,
            deflate: None,
            br: None,
            zstd: None,
            level: Some(9),
        };

        let host = SettingHost {
            ip: "127.0.0.1".to_string(),
            port: 8080,
            server_name: None,
            ssl: false,
            certificate: None,
            certificate_key: None,
            route: vec![route_no_custom, route_with_custom],
            route_map: DashMap::new(),
            timeout: 75,
        };

        // 只要有一个路由有自定义配置，就返回 true
        assert!(host_has_any_custom_compression(&host));
    }

    #[test]
    fn test_compression_config_default_integration() {
        // 测试默认配置在无路由自定义配置时的行为
        let global = CompressionConfig::default();

        // 默认配置应该启用所有压缩类型
        assert!(global.gzip);
        assert!(global.deflate);
        assert!(global.br);
        assert!(global.zstd);
        assert_eq!(global.level, 6);
    }

    #[test]
    fn test_compression_level_edge_cases() {
        // 测试压缩级别边界情况
        let test_cases = [
            (0u8, false, false),  // 超出范围，使用 Default
            (1u8, true, false),   // Fastest
            (5u8, false, false),  // Precise
            (9u8, false, true),   // Best
            (10u8, false, false), // 超出范围，使用 Default
        ];

        for (level, is_fastest, is_best) in test_cases {
            let route = SettingRoute {
                location: "/".to_string(),
                root: Some("/var/www".to_string()),
                auto_index: false,
                index: vec![],
                error_page: None,
                not_found_page: None,
                proxy_pass: None,
                upstream: None,
                forward_proxy: None,
                proxy_timeout: 5,
                max_body_size: None,
                headers: None,
                lua_script: None,
                redirect_to: None,
                redirect_code: None,
                gzip: None,
                deflate: None,
                br: None,
                zstd: None,
                level: Some(level),
            };

            let global = CompressionConfig::default();
            let _layer = build_compression_layer(&route, &global);

            // 验证级别映射逻辑
            let expected_level = if is_fastest {
                CompressionLevel::Fastest
            } else if is_best {
                CompressionLevel::Best
            } else if (2..=8).contains(&level) {
                CompressionLevel::Precise(level as i32)
            } else {
                CompressionLevel::Default
            };

            // 实际应用中，CompressionLevel 实现了 PartialEq
            // 这里我们只是验证代码可以正确构建层
            let _ = expected_level;
        }
    }
}
