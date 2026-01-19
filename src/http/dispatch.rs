use axum::{
    Router, async_trait,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
    response::IntoResponse,
    routing::get,
};
use dashmap::DashMap;
use lazy_static::lazy_static;
use std::sync::Arc;

use crate::config::SettingHost;

/// 域名路由调度中间件
/// 根据请求的 Host 头部将请求路由到对应的域名配置
pub async fn dispatch_request(parts: &mut Parts) -> Option<SettingHost> {
    // 从 Host 头部获取域名信息
    let host_header = parts.headers.get("Host")?.to_str().ok()?;
    let (domain, port_str) = host_header.split_once(':').unwrap_or((host_header, ""));
    let port = port_str.parse::<u16>().unwrap_or_else(|_| {
        // 根据协议推断默认端口
        if parts.uri.scheme_str() == Some("https") {
            443
        } else {
            80
        }
    });

    // 查找端口对应的域名配置
    let port_config = crate::http::HOSTS.get(&port)?;

    // 查找匹配的域名配置
    let domain_lower = domain.to_lowercase();
    if let Some(entry) = port_config.get(&Some(domain_lower.clone())) {
        return Some(entry.clone());
    }

    // 尝试不区分大小写的匹配
    for entry in port_config.iter() {
        if let Some(server_name) = entry.key() {
            if server_name.to_lowercase() == domain_lower {
                return Some(entry.value().clone());
            }
        }
    }

    // 查找默认主机配置（无域名）
    port_config.get(&None).cloned()
}
