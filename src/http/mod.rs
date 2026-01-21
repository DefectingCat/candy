use std::{net::SocketAddr, sync::LazyLock, time::Duration};

use anyhow::anyhow;
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use dashmap::DashMap;
use http::StatusCode;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::{debug, info, warn};

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

pub async fn make_server(host: SettingHost) -> anyhow::Result<axum_server::Handle<SocketAddr>> {
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

    let addr = format!("{}:{}", host.ip, host.port);
    let addr: SocketAddr = addr.parse()?;

    let handle = Handle::new();
    let handle_clone = handle.clone();

    // 生成一个任务来运行服务器
    tokio::spawn(async move {
        // 检查是否启用 SSL
        // 如果启用 SSL
        // 则创建 SSL 监听器
        // 否则创建 TCP 监听器
        let result = if host.ssl && host.certificate.is_some() && host.certificate_key.is_some() {
            let cert = host
                .certificate
                .as_ref()
                .ok_or(anyhow!("Certificate not found"))?;
            let key = host
                .certificate_key
                .as_ref()
                .ok_or(anyhow!("Certificate key not found"))?;
            debug!("Certificate: {} Certificate key: {}", cert, key);

            let rustls_config = RustlsConfig::from_pem_file(cert, key).await?;
            info!("Listening on https://{}", addr);
            axum_server::bind_rustls(addr, rustls_config)
                .handle(handle_clone.clone())
                .serve(router.into_make_service())
                .await
        } else {
            info!("Listening on http://{}", addr);
            axum_server::bind(addr)
                .handle(handle_clone.clone())
                .serve(router.into_make_service())
                .await
        };

        result.map_err(|e| anyhow::Error::from(e))
    });

    Ok(handle)
}
