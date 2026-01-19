use std::sync::Arc;

use axum::{
    Router, async_trait,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::IntoResponse,
    routing::get,
};
use dashmap::DashMap;

use crate::config::SettingHost;

use super::serve;

/// 域名路由调度中间件
/// 根据请求的 Host 头部将请求路由到对应的域名配置
pub async fn domain_router(
    port: u16,
    domain_configs: Arc<DashMap<Option<String>, SettingHost>>,
) -> Router {
    let mut router = Router::new();

    // 为每个域名创建独立的路由
    for entry in domain_configs.iter() {
        let domain = entry.key().clone();
        let host_config = entry.value().clone();

        // 创建该域名的路由
        let mut domain_router = Router::new();
        for host_route in &host_config.route {
            // 这里可以根据 route 类型注册不同的处理函数
            // 目前简单起见，我们只处理静态文件服务
            domain_router = domain_router.route(
                &format!("{}{{*path}}", host_route.location),
                get(serve::serve),
            );
        }

        // 为该域名设置路由前缀或使用中间件
        // 这里我们使用一个中间件来检查 Host 头部
        router = router.route_layer(middleware::from_fn(move |req, next| {
            check_domain(domain.clone(), req, next)
        }));
    }

    router
}

/// 检查请求的 Host 头部是否与配置的域名匹配
async fn check_domain<B>(
    expected_domain: Option<String>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let host = req
        .headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .map(|h| {
            // 去除端口号
            h.split(':').next().unwrap_or(h).to_lowercase()
        });

    // 检查域名是否匹配
    if let Some(expected) = expected_domain {
        if let Some(actual) = host {
            if actual != expected.to_lowercase() {
                return Err(StatusCode::NOT_FOUND);
            }
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    Ok(next.run(req).await)
}
