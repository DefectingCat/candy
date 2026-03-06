//! HTTP 重定向处理模块
//!
//! 提供 HTTP 重定向功能，支持 301（永久重定向）和 302（临时重定向）。
//!
//! # 功能特性
//!
//! - HTTP 重定向：支持 301 和 302 状态码
//! - 自定义重定向 URL：支持配置任意目标 URL
//! - 基于域名的虚拟主机：支持不同域名使用不同的重定向配置
//!
//! # 配置示例
//!
//! ```toml
//! [[host.route]]
//! location = "/old-path"
//! redirect_to = "https://example.com/new-path"
//! redirect_code = 301
//! ```
//!
//! # 重定向流程
//!
//! 1. 解析请求的 URI 和主机配置
//! 2. 在路由映射中查找重定向配置
//! 3. 构建重定向响应，设置 Location 头
//! 4. 返回重定向状态码（默认 301）
//!
//! # 使用示例
//!
//! ```no_run
//! use candy::http::redirect::redirect;
//! use axum::Router;
//!
//! // 路由会自动注册重定向处理器
//! let app = Router::new().route("/old", get(redirect));
//! ```

use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use http::{Uri, header::LOCATION};
use tracing::debug;

use crate::{
    http::{
        HOSTS,
        error::{RouteError, RouteResult},
        serve::resolve_parent_path,
    },
    utils::parse_port_from_host,
};

/// 处理 HTTP 重定向请求
///
/// 根据配置将请求重定向到指定的目标 URL。
///
/// # 参数
///
/// * `req_uri` - 请求的 URI
/// * `path` - 可选的路径参数（由路由器提取）
/// * `req` - HTTP 请求对象
///
/// # 返回值
///
/// 返回重定向响应，包含：
/// - 状态码：301（永久重定向，默认）或 302（临时重定向）
/// - Location 头：目标 URL
///
/// # 错误
///
/// - `RouteError::BadRequest()` - 请求格式错误或主机未找到
/// - `RouteError::RouteNotFound()` - 路由配置未找到
/// - `RouteError::InternalError()` - 重定向配置缺失
///
/// # 示例
///
/// ```toml
/// # 配置示例
/// [[host.route]]
/// location = "/old"
/// redirect_to = "https://example.com/new"
/// redirect_code = 301
/// ```
///
/// # 注意
///
/// - 重定向状态码默认为 301
/// - 支持基于域名的虚拟主机配置
/// - 域名匹配不区分大小写
pub async fn redirect(
    req_uri: Uri,
    path: Option<Path<String>>,
    req: Request<Body>,
) -> RouteResult<impl IntoResponse> {
    let scheme = req.uri().scheme_str().unwrap_or("http");
    let host = req
        .headers()
        .get("host") // 注意：host 是小写的
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    let port = parse_port_from_host(host, scheme).ok_or(RouteError::BadRequest())?;
    // 解析域名
    let (domain, _) = host.split_once(':').unwrap_or((host, ""));
    let domain = domain.to_lowercase();

    let host_config = {
        let port_config = HOSTS.get(&port).ok_or(RouteError::BadRequest())?;

        // 查找匹配的域名配置
        let host_config = if let Some(entry) = port_config.get(&Some(domain.clone())) {
            Some(entry.clone())
        } else {
            // 尝试不区分大小写的匹配
            let mut found = None;
            for entry in port_config.iter() {
                if let Some(server_name) = entry.key()
                    && server_name.to_lowercase() == domain
                {
                    found = Some(entry.value().clone());
                    break;
                }
            }
            found.or_else(|| port_config.get(&None).map(|v| v.clone()))
        };

        host_config.ok_or(RouteError::BadRequest())?
    };

    let route_map = &host_config.route_map;
    debug!("Redirect: Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    let route_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())?;

    let Some(redirect_to) = route_config.redirect_to.as_ref() else {
        return Err(RouteError::InternalError());
    };

    let redirect_code = route_config.redirect_code.unwrap_or(301);
    let mut response = Response::builder();
    response = response.status(redirect_code);
    response = response.header(LOCATION, redirect_to);
    Ok(response
        .body(Body::empty())
        .with_context(|| "Failed to build HTTP response with body in HTTP redirect")?)
}
