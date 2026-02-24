use std::{net::SocketAddr, sync::LazyLock, time::Duration};

use std::sync::Arc;

use anyhow::anyhow;
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use dashmap::DashMap;
use http::StatusCode;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::{debug, error, info, warn};

use crate::{
    config::{SettingHost, Upstream},
    middlewares::{add_headers, add_version, logging_route},
};

/// 上游服务器组配置存储
/// 使用 upstream 名称作为键，Upstream 配置作为值
pub static UPSTREAMS: LazyLock<DashMap<String, Upstream>> = LazyLock::new(DashMap::new);

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
    }
}

/// 启动初始服务器实例
pub async fn start_initial_servers(
    settings: crate::config::Settings,
) -> anyhow::Result<Arc<Mutex<Vec<axum_server::Handle<SocketAddr>>>>> {
    let handles = start_servers(settings.host).await;
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
                }

                let new_handles = start_servers(new_hosts).await;

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
///
/// # 返回值
///
/// 返回一个包含所有成功启动的服务器句柄的向量
///
/// # 错误处理
///
/// 单个服务器启动失败会被捕获并记录为错误日志，不会影响其他服务器的启动
pub async fn start_servers(hosts: Vec<SettingHost>) -> Vec<axum_server::Handle<SocketAddr>> {
    let mut handles = Vec::new();
    for host in hosts {
        match make_server(host).await {
            Ok(handle) => {
                handles.push(handle);
                info!("Server instance started");
            }
            Err(e) => {
                error!("Failed to start server instance: {:?}", e);
            }
        }
    }
    handles
}

pub async fn make_server(host: SettingHost) -> anyhow::Result<axum_server::Handle<SocketAddr>> {
    debug!("make_server start with host: {:?}", host);
    let mut router = Router::new();
    let host_to_save = host.clone();

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
            let (new_router, route_path) =
                register_route(router, &host_route.location, get(redirect::redirect));
            router = new_router;

            // 将路由路径保存到映射中
            host_to_save
                .route_map
                .insert(route_path.clone(), host_route.clone());

            let wildcard_path = format!("{route_path}{{*path}}");
            router = router.route(&wildcard_path, get(serve::serve));
            debug!("HTTP redirect wildcard route registered: {}", wildcard_path);
            continue;
        }

        // Lua 脚本
        #[cfg(feature = "lua")]
        if host_route.lua_script.is_some() {
            router = router.route(&host_route.location, get(lua::lua));
            let wildcard_path = format!("{}{{*path}}", host_route.location);
            router = router.route(&wildcard_path, get(lua::lua));

            host_to_save
                .route_map
                .insert(host_route.location.clone(), host_route.clone());
            debug!("Lua script route registered: {}", wildcard_path);
            continue;
        }

        // 反向代理（包括 upstream 负载均衡）
        if host_route.proxy_pass.is_some() || host_route.upstream.is_some() {
            router = router.route(&host_route.location, get(reverse_proxy::serve));
            let wildcard_path = format!("{}{{*path}}", host_route.location);
            router = router.route(&wildcard_path, get(reverse_proxy::serve));

            if let Some(max_body_size) = host_route.max_body_size {
                router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
            }

            host_to_save
                .route_map
                .insert(host_route.location.clone(), host_route.clone());
            debug!("Reverse proxy route registered: {}", wildcard_path);
            continue;
        }

        // 正向代理
        if host_route.forward_proxy.is_some() && host_route.forward_proxy.unwrap() {
            router = router.route(&host_route.location, get(forward_proxy::serve));
            let wildcard_path = format!("{}{{*path}}", host_route.location);
            router = router.route(&wildcard_path, get(forward_proxy::serve));

            if let Some(max_body_size) = host_route.max_body_size {
                router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
            }

            host_to_save
                .route_map
                .insert(host_route.location.clone(), host_route.clone());
            debug!("Forward proxy route registered: {}", wildcard_path);
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

        let (new_router, route_path) =
            register_route(router, &host_route.location, get(serve::serve));
        router = new_router;

        host_to_save
            .route_map
            .insert(route_path.clone(), host_route.clone());

        let wildcard_path = format!("{route_path}{{*path}}");
        router = router.route(&wildcard_path, get(serve::serve));
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

    router = router.layer(
        ServiceBuilder::new()
            .layer(middleware::from_fn(add_version))
            .layer(middleware::from_fn(add_headers))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::SERVICE_UNAVAILABLE,
                Duration::from_secs(host.timeout.into()),
            ))
            .layer(CompressionLayer::new()),
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
