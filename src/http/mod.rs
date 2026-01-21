use std::{net::SocketAddr, sync::LazyLock, time::Duration};

use anyhow::anyhow;
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use dashmap::DashMap;
use http::StatusCode;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::{debug, error, info, warn};

use crate::{
    config::SettingHost,
    middlewares::{add_headers, add_version, logging_route},
};

pub mod error;
// 处理静态文件
pub mod serve;
// 处理反向代理
pub mod reverse_proxy;
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
    // 在配置中查找路由
    // 转换为 Axum 路由
    // 注册路由
    for host_route in &host.route {
        // HTTP 重定向
        if host_route.redirect_to.is_some() {
            // 使用位置注册
            // location = "/doc"
            // 路由: GET /doc/*
            // 使用文件路径注册
            // index = ["index.html", "index.txt"]
            // 路由: GET /doc/index.html
            // 路由: GET /doc/index.txt
            // 注册父路径 /doc
            let path_morethan_one = host_route.location.len() > 1;
            let route_path = if path_morethan_one && host_route.location.ends_with('/') {
                // 首先注册带斜杠的路径 /doc
                router = router.route(&host_route.location, get(redirect::redirect));
                debug!("Route registered: {}", host_route.location);
                let len = host_route.location.len();
                let path_without_slash = host_route.location.chars().collect::<Vec<_>>()
                    [0..len - 1]
                    .iter()
                    .collect::<String>();
                // 然后注册不带斜杠的路径 /doc/
                router = router.route(&path_without_slash, get(redirect::redirect));
                debug!("Route registered: {}", path_without_slash);
                host_route.location.clone()
            } else if path_morethan_one {
                // 首先注册不带斜杠的路径 /doc
                router = router.route(&host_route.location, get(redirect::redirect));
                debug!("Route registered: {}", host_route.location);
                // 然后注册带斜杠的路径 /doc/
                let path = format!("{}/", host_route.location);
                router = router.route(&path, get(redirect::redirect));
                debug!("Route registered: {}", path);
                path
            } else {
                // 注册路径 /doc/
                router = router.route(&host_route.location, get(serve::serve));
                debug!("Route registered: {}", host_route.location);
                host_route.location.clone()
            };
            // 将路由路径保存到映射中
            {
                host_to_save
                    .route_map
                    .insert(route_path.clone(), host_route.clone());
            }
            let route_path = format!("{route_path}{{*path}}");
            // 注册通配符路径 /doc/*
            router = router.route(route_path.as_ref(), get(serve::serve));
            debug!("HTTP redirect route registered: {}", route_path);
            continue;
        }

        // Lua 脚本
        #[cfg(feature = "lua")]
        if host_route.lua_script.is_some() {
            // 准备 Lua 脚本
            router = router.route(host_route.location.as_ref(), get(lua::lua));
            let route_path = format!("{}{{*path}}", host_route.location);
            router = router.route(route_path.as_ref(), get(lua::lua));
            // 将路由路径保存到映射中
            {
                host_to_save
                    .route_map
                    .insert(host_route.location.clone(), host_route.clone());
            }
            debug!("Lua script route registered: {}", route_path);
            continue;
        }

        // 反向代理
        if host_route.proxy_pass.is_some() {
            router = router.route(host_route.location.as_ref(), get(reverse_proxy::serve));
            // 注册通配符路径 /doc/*
            let route_path = format!("{}{{*path}}", host_route.location);
            router = router.route(route_path.as_ref(), get(reverse_proxy::serve));
            // 设置请求最大体大小
            if let Some(max_body_size) = host_route.max_body_size {
                router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
            }
            // 将路由路径保存到映射中
            {
                host_to_save
                    .route_map
                    .insert(host_route.location.clone(), host_route.clone());
            }
            debug!("Reverse proxy route registered: {}", route_path);
            continue;
        }

        // 静态文件
        if host_route.root.is_none() {
            warn!("Route missing root field: {:?}", host_route.location);
            continue;
        }
        // 设置请求最大体大小
        if let Some(max_body_size) = host_route.max_body_size {
            router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
        }
        // 使用位置注册
        // location = "/doc"
        // 路由: GET /doc/*
        // 使用文件路径注册
        // index = ["index.html", "index.txt"]
        // 路由: GET /doc/index.html
        // 路由: GET /doc/index.txt
        // 注册父路径 /doc
        let path_morethan_one = host_route.location.len() > 1;
        let route_path = if path_morethan_one && host_route.location.ends_with('/') {
            // 首先注册带斜杠的路径 /doc
            router = router.route(&host_route.location, get(serve::serve));
            debug!("Route registered: {}", host_route.location);
            let len = host_route.location.len();
            let path_without_slash = host_route.location.chars().collect::<Vec<_>>()[0..len - 1]
                .iter()
                .collect::<String>();
            // 然后注册不带斜杠的路径 /doc/
            router = router.route(&path_without_slash, get(serve::serve));
            debug!("Route registered: {}", path_without_slash);
            host_route.location.clone()
        } else if path_morethan_one {
            // 首先注册不带斜杠的路径 /doc
            router = router.route(&host_route.location, get(serve::serve));
            debug!("Route registered: {}", host_route.location);
            // 然后注册带斜杠的路径 /doc/
            let path = format!("{}/", host_route.location);
            router = router.route(&path, get(serve::serve));
            debug!("Route registered: {}", path);
            path
        } else {
            // 注册路径 /doc/
            router = router.route(&host_route.location, get(serve::serve));
            debug!("Route registered: {}", host_route.location);
            host_route.location.clone()
        };
        // 将路由路径保存到映射中
        {
            host_to_save
                .route_map
                .insert(route_path.clone(), host_route.clone());
        }
        let route_path = format!("{route_path}{{*path}}");
        // 注册通配符路径 /doc/*
        router = router.route(route_path.as_ref(), get(serve::serve));
        debug!("Static file route registered: {}", route_path);
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
        // 检查是否启用 SSL
        // 如果启用 SSL
        // 则创建 SSL 监听器
        // 否则创建 TCP 监听器
        let result = if ssl && certificate.is_some() && certificate_key.is_some() {
            let cert = certificate
                .as_ref()
                .ok_or(anyhow!("Certificate not found"))?;
            let key = certificate_key
                .as_ref()
                .ok_or(anyhow!("Certificate key not found"))?;
            debug!("Certificate: {} Certificate key: {}", cert, key);

            let rustls_config = RustlsConfig::from_pem_file(cert, key).await?;
            info!("Listening on https://{}", addr);
            axum_server::bind_rustls(addr, rustls_config)
                .handle(handle_clone)
                .serve(router.into_make_service())
                .await
        } else {
            info!("Listening on http://{}", addr);
            axum_server::bind(addr)
                .handle(handle_clone)
                .serve(router.into_make_service())
                .await
        };

        result.map_err(anyhow::Error::from)
    });

    Ok(handle)
}
