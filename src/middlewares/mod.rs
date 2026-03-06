//! HTTP 中间件模块
//!
//! 提供 Axum 服务器的中间件功能，用于处理请求和响应。
//!
//! # 中间件列表
//!
//! - `add_version`：添加服务器版本信息到响应头
//! - `add_headers`：添加自定义响应头
//! - `logging_route`：请求日志记录中间件
//!
//! # 中间件执行顺序
//!
//! 中间件按照添加顺序执行（洋葱模型）：
//!
//! 1. **请求阶段**：从外到内执行
//!    - logging_route（记录请求开始）
//!    - add_version（准备添加版本信息）
//!    - add_headers（准备添加自定义头部）
//!
//! 2. **响应阶段**：从内到外执行
//!    - add_headers（添加自定义头部到响应）
//!    - add_version（添加版本信息到响应）
//!    - logging_route（记录请求完成和延迟）
//!
//! # 使用示例
//!
//! ```no_run
//! use candy::middlewares::{add_version, add_headers, logging_route};
//! use axum::Router;
//!
//! let app = Router::new()
//!     .layer(axum::middleware::from_fn(add_version))
//!     .layer(axum::middleware::from_fn(add_headers));
//!
//! let app = logging_route(app);
//! ```

use std::{fmt::Display, time::Duration};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, Request},
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http::HeaderName;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::TraceLayer;
use tracing::{Span, debug, error, info, info_span};

use crate::{
    consts::{NAME, VERSION},
    http::{HOSTS, serve::resolve_parent_path},
    utils::parse_port_from_host,
};

/// 添加服务器版本信息到响应头
///
/// 此中间件会在每个 HTTP 响应的头部添加以下信息：
/// - `Server`：服务器名称（Candy）
/// - `RUA-Version`：服务器版本号
///
/// # 参数
///
/// * `req` - HTTP 请求对象
/// * `next` - 下一个中间件或路由处理器
///
/// # 返回值
///
/// 返回添加了版本信息的 HTTP 响应。
///
/// # 示例
///
/// ```no_run
/// use candy::middlewares::add_version;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(axum::middleware::from_fn(add_version));
/// ```
///
/// # 响应头示例
///
/// ```http
/// HTTP/1.1 200 OK
/// Server: Candy
/// RUA-Version: 0.2.5
/// ```
pub async fn add_version(req: Request<Body>, next: Next) -> impl IntoResponse {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.append("Server", HeaderValue::from_static(NAME));
    headers.append("RUA-Version", HeaderValue::from_static(VERSION));
    res
}

/// 动态添加自定义响应头
///
/// 根据主机配置动态添加自定义 HTTP 响应头。
/// 支持基于域名和路由的头部配置。
///
/// # 参数
///
/// * `req` - HTTP 请求对象
/// * `next` - 下一个中间件或路由处理器
///
/// # 返回值
///
/// 返回添加了自定义头部的 HTTP 响应。
///
/// # 工作流程
///
/// 1. 从请求中提取 `Host` 头
/// 2. 解析主机名和端口
/// 3. 在全局配置中查找对应的主机配置
/// 4. 从路由配置中获取自定义头部
/// 5. 将自定义头部添加到响应中
///
/// # 配置示例
///
/// ```toml
/// [[host]]
/// ip = "0.0.0.0"
/// port = 8080
///
/// [[host.route]]
/// location = "/"
/// root = "./html"
/// headers = { "X-Custom-Header" = "value", "X-Another-Header" = "another-value" }
/// ```
///
/// # 错误处理
///
/// - 如果 `Host` 头缺失或格式错误，跳过头部添加
/// - 如果端口无效或主机配置未找到，跳过头部添加
/// - 如果头部名称或值格式错误，记录错误日志并跳过该头部
///
/// # 示例
///
/// ```no_run
/// use candy::middlewares::add_headers;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(axum::middleware::from_fn(add_headers));
/// ```
pub async fn add_headers(req: Request, next: Next) -> impl IntoResponse {
    let scheme = req.uri().scheme_str().unwrap_or("http");
    let host_header = req
        .headers()
        .get("host") // 注意：host 是小写的
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_string();
    debug!("scheme {:?}", scheme);
    let Some(port) = parse_port_from_host(&host_header, scheme) else {
        return next.run(req).await;
    };
    let uri = req.uri();
    let path = req.extensions().get::<Path<String>>();
    let parent_path = resolve_parent_path(uri, path);

    debug!("port {:?}", port);
    let mut res = next.run(req).await;
    let req_headers = res.headers_mut();
    let Some(port_config) = HOSTS.get(&port) else {
        return res;
    };

    // 解析域名
    let (domain, _) = host_header.split_once(':').unwrap_or((&host_header, ""));
    let domain = domain.to_lowercase();

    // 查找匹配的域名配置
    let host = if let Some(entry) = port_config.get(&Some(domain.clone())) {
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

    let Some(host) = host else {
        return res;
    };

    let route_map = &host.route_map;

    // Find host route
    let Some(host_route) = route_map.get(&parent_path) else {
        return res;
    };
    let Some(headers) = &host_route.headers else {
        return res;
    };

    headers.iter().for_each(|entry| {
        let (key, value) = (entry.key(), entry.value());
        let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) else {
            error!("Invalid header name: {key}");
            return;
        };
        let Ok(header_value) = HeaderValue::from_bytes(value.as_bytes()) else {
            error!("Invalid header value: {value}");
            return;
        };
        req_headers.append(header_name, header_value);
    });
    res
}

/// HTTP 请求日志记录中间件
///
/// 为每个 HTTP 请求添加详细的日志记录，包括：
/// - 请求方法、URI、主机、User-Agent
/// - 响应状态码
/// - 请求处理延迟
///
/// # 参数
///
/// * `router` - Axum 路由器实例
///
/// # 返回值
///
/// 返回添加了日志中间件的路由器。
///
/// # 日志格式
///
/// ```text
/// 2024-01-01T12:00:00Z INFO HTTP method=GET host="example.com" uri="/" ua="Mozilla/5.0"
/// 2024-01-01T12:00:00Z INFO 200 OK 15ms
/// ```
///
/// # 日志级别
///
/// - 成功响应（2xx-4xx）：INFO 级别
/// - 服务器错误（5xx）：ERROR 级别
///
/// # 性能指标
///
/// - 延迟小于 1ms：显示微秒（μs）
/// - 延迟大于等于 1ms：显示毫秒（ms）
///
/// # 示例
///
/// ```no_run
/// use candy::middlewares::logging_route;
/// use axum::Router;
///
/// let app = Router::new();
/// let app = logging_route(app);
/// ```
///
/// # 实现细节
///
/// 使用 `tower_http::TraceLayer` 实现，支持：
/// - 自定义 span 创建（包含请求信息）
/// - 响应完成时记录延迟
/// - 错误时记录失败信息
pub fn logging_route(router: Router) -> Router {
    let make_span = |req: &Request<_>| {
        let unknown = &HeaderValue::from_static("Unknown");
        let empty = &HeaderValue::from_static("");
        let headers = req.headers();
        let ua = headers
            .get("User-Agent")
            .unwrap_or(unknown)
            .to_str()
            .unwrap_or("Unknown");
        let host = headers.get("Host").unwrap_or(empty).to_str().unwrap_or("");
        info_span!("HTTP", method = ?req.method(), host, uri = ?req.uri(), ua)
    };

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(make_span)
        .on_request(|_req: &Request<_>, _span: &Span| {})
        .on_response(|res: &Response, latency: Duration, _span: &Span| {
            info!("{}", format_latency(latency, res.status()));
        })
        .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {})
        .on_eos(|_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {})
        .on_failure(
            |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                error!("{}", format_latency(latency, error));
            },
        );

    router.layer(trace_layer)
}

/// 格式化请求延迟时间和状态信息
///
/// 根据延迟时间的大小，自动选择合适的单位（微秒或毫秒）进行格式化。
///
/// # 参数
///
/// * `latency` - 请求处理延迟时间
/// * `status` - HTTP 状态码或错误类型（实现 `Display` trait）
///
/// # 返回值
///
/// 返回格式化后的字符串，格式为："{status} {latency}{unit}"
///
/// # 单位选择
///
/// - 延迟 < 1ms（1000μs）：显示微秒（μs）
/// - 延迟 >= 1ms：显示毫秒（ms）
///
/// # 示例
///
/// ```
/// use candy::middlewares::format_latency;
/// use std::time::Duration;
/// use http::StatusCode;
///
/// // 小于 1ms
/// let latency = Duration::from_micros(500);
/// assert_eq!(format_latency(latency, StatusCode::OK), "200 OK 500μs");
///
/// // 大于等于 1ms
/// let latency = Duration::from_millis(15);
/// assert_eq!(format_latency(latency, StatusCode::OK), "200 OK 15ms");
///
/// // 使用错误状态
/// let latency = Duration::from_millis(50);
/// assert_eq!(format_latency(latency, StatusCode::NOT_FOUND), "404 Not Found 50ms");
/// ```
fn format_latency(latency: Duration, status: impl Display) -> String {
    let micros = latency.as_micros();
    let millis = latency.as_millis();
    if micros >= 1000 {
        format!("{status} {millis}ms")
    } else {
        format!("{status} {micros}μs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use std::time::Duration;

    #[test]
    fn test_format_latency_microseconds() {
        // 测试小于 1ms 的延迟
        let latency = Duration::from_micros(500);
        let result = format_latency(latency, StatusCode::OK);
        assert_eq!(result, "200 OK 500μs");
    }

    #[test]
    fn test_format_latency_milliseconds() {
        // 测试大于等于 1ms 的延迟
        let latency = Duration::from_millis(250);
        let result = format_latency(latency, StatusCode::OK);
        assert_eq!(result, "200 OK 250ms");
    }

    #[test]
    fn test_format_latency_one_millisecond() {
        // 测试正好 1ms 的延迟
        let latency = Duration::from_millis(1);
        let result = format_latency(latency, StatusCode::OK);
        assert_eq!(result, "200 OK 1ms");
    }

    #[test]
    fn test_format_latency_with_error_status() {
        // 测试错误状态码
        let latency = Duration::from_micros(800);
        let result = format_latency(latency, StatusCode::NOT_FOUND);
        assert_eq!(result, "404 Not Found 800μs");
    }

    #[test]
    fn test_format_latency_large_value() {
        // 测试较大的延迟值
        let latency = Duration::from_secs(2);
        let result = format_latency(latency, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(result, "500 Internal Server Error 2000ms");
    }
}
