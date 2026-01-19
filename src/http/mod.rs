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
    utils::graceful_shutdown,
};

pub mod error;
// 处理静态文件
pub mod serve;
// 处理反向代理
pub mod reverse_proxy;
// 处理 Lua 脚本
pub mod lua;
// 处理 HTTP 重定向
pub mod redirect;

// 0.2.4 待办
// 主机配置更新以支持域名
// {
//     80: {
//         "rua.plus": {
//             "/doc": <SettingRoute>
//         }
//         "www.rua.plus": {
//             "/doc": <SettingRoute>
//         }
//     }
// }

/// 主机配置
/// 使用虚拟主机端口作为键
/// 使用 SettingHost 作为值
/// 使用端口作为父级部分
/// 使用 host.route.location 作为键
/// 使用 host.route 结构体作为值
/// {
///     80: {
///         "/doc": <SettingRoute>
///     }
/// }
pub static HOSTS: LazyLock<DashMap<u16, SettingHost>> = LazyLock::new(DashMap::new);

pub async fn make_server(host: SettingHost) -> anyhow::Result<()> {
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
                debug!("已注册路由 {}", host_route.location);
                let len = host_route.location.len();
                let path_without_slash = host_route.location.chars().collect::<Vec<_>>()
                    [0..len - 1]
                    .iter()
                    .collect::<String>();
                // 然后注册不带斜杠的路径 /doc/
                router = router.route(&path_without_slash, get(redirect::redirect));
                debug!("已注册路由 {}", path_without_slash);
                host_route.location.clone()
            } else if path_morethan_one {
                // 首先注册不带斜杠的路径 /doc
                router = router.route(&host_route.location, get(redirect::redirect));
                debug!("已注册路由 {}", host_route.location);
                // 然后注册带斜杠的路径 /doc/
                let path = format!("{}/", host_route.location);
                router = router.route(&path, get(redirect::redirect));
                debug!("已注册路由 {}", path);
                path
            } else {
                // 注册路径 /doc/
                router = router.route(&host_route.location, get(serve::serve));
                debug!("已注册路由 {}", host_route.location);
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
            debug!("已注册 HTTP 重定向路由: {}", route_path);
            continue;
        }

        // Lua 脚本
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
            debug!("已注册 Lua 脚本路由: {}", route_path);
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
            debug!("已注册反向代理路由: {}", route_path);
            continue;
        }

        // 静态文件
        if host_route.root.is_none() {
            warn!("路由未找到 root 字段: {:?}", host_route.location);
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
            debug!("已注册路由 {}", host_route.location);
            let len = host_route.location.len();
            let path_without_slash = host_route.location.chars().collect::<Vec<_>>()[0..len - 1]
                .iter()
                .collect::<String>();
            // 然后注册不带斜杠的路径 /doc/
            router = router.route(&path_without_slash, get(serve::serve));
            debug!("已注册路由 {}", path_without_slash);
            host_route.location.clone()
        } else if path_morethan_one {
            // 首先注册不带斜杠的路径 /doc
            router = router.route(&host_route.location, get(serve::serve));
            debug!("已注册路由 {}", host_route.location);
            // 然后注册带斜杠的路径 /doc/
            let path = format!("{}/", host_route.location);
            router = router.route(&path, get(serve::serve));
            debug!("已注册路由 {}", path);
            path
        } else {
            // 注册路径 /doc/
            router = router.route(&host_route.location, get(serve::serve));
            debug!("已注册路由 {}", host_route.location);
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
        debug!("已注册静态文件路由: {}", route_path);
    }

    // 保存主机到映射中
    HOSTS.insert(host.port, host_to_save);

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
    // 生成一个任务来优雅地关闭服务器
    tokio::spawn(graceful_shutdown(handle.clone()));

    // 检查是否启用 SSL
    // 如果启用 SSL
    // 则创建 SSL 监听器
    // 否则创建 TCP 监听器
    if host.ssl && host.certificate.is_some() && host.certificate_key.is_some() {
        let cert = host.certificate.as_ref().ok_or(anyhow!("未找到证书"))?;
        let key = host
            .certificate_key
            .as_ref()
            .ok_or(anyhow!("未找到证书密钥"))?;
        debug!("证书 {} 证书密钥 {}", cert, key);

        let rustls_config = RustlsConfig::from_pem_file(cert, key).await?;
        info!("正在监听 https://{}", addr);
        axum_server::bind_rustls(addr, rustls_config)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
    } else {
        info!("正在监听 http://{}", addr);
        axum_server::bind(addr)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
    }

    Ok(())
}
