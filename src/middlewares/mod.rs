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

/// Middleware for adding version information to each response's headers.
///
/// This middleware takes an incoming `Request` and a `Next` handler, which represents the
/// subsequent middleware or route in the chain. It then asynchronously runs the next handler,
/// obtaining the response. After receiving the response, it appends two headers:
/// - "Server": The name of the server extracted from the Cargo package name.
/// - "S-Version": The version of the server extracted from the Cargo package version.
pub async fn add_version(req: Request<Body>, next: Next) -> impl IntoResponse {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.append("Server", HeaderValue::from_static(NAME));
    headers.append("RUA-Version", HeaderValue::from_static(VERSION));
    res
}

/// Middleware for dynamically adding headers to responses based on the requested host and port.
///
/// This middleware:
/// 1. Extracts the `Host` header from the incoming request.
/// 2. Parses the host string to determine the port (defaulting to `80` if unspecified).
/// 3. Looks up the host configuration in the global `HOST` map (shared state) for the resolved port.
/// 4. Appends any configured headers from the host's `SettingHost` to the response.
///
/// # Behavior
/// - If the `Host` header is missing or malformed, the request proceeds unchanged.
/// - If the port is invalid or the host configuration is not found, the request proceeds unchanged.
/// - Headers are appended to the response only if they are explicitly configured for the host.
///
/// # Error Handling
/// - Silently skips header addition for:
///   - Missing or unparseable `Host` headers.
///   - Invalid ports (non-numeric or out-of-range).
///   - Missing host configurations in `HOST`.
/// - Uses `debug!` for logging the resolved port.
///
/// # Example
/// Given a request to `example.com:8080` and a `HOST` entry for port `8080` with headers:
/// ```toml
/// [hosts."8080"]
/// headers = { "X-Custom" = "value" }
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

/// Middleware for logging each request.
///
/// This middleware will calculate each request latency
/// and add request's information to each info_span.
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

/// Format request latency and status message
/// return a string
fn format_latency(latency: Duration, status: impl Display) -> String {
    let micros = latency.as_micros();
    let millis = latency.as_millis();
    if micros >= 1000 {
        format!("{status} {millis}ms")
    } else {
        format!("{status} {micros}μs")
    }
}
