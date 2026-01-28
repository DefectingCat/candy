use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

use dashmap::DashMap;

use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use http::{HeaderName, Uri};
use reqwest::Client;

use super::{
    HOSTS, UPSTREAMS,
    error::{RouteError, RouteResult},
};
use crate::http::serve::custom_page;
use crate::{http::serve::resolve_parent_path, utils::parse_port_from_host};

/// 加权轮询计数器存储
/// 用于跟踪每个 upstream 的当前轮询权重和索引
static WEIGHTED_ROUND_ROBIN_COUNTERS: LazyLock<DashMap<String, AtomicUsize>> =
    LazyLock::new(DashMap::new);

/// 全局 reqwest 客户端实例，用于复用连接池，提高性能
static CLIENT: OnceLock<Client> = OnceLock::new();

/// 获取全局 reqwest 客户端实例
fn get_client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .build()
            .expect("Failed to initialize reqwest client")
    })
}

/// 处理入站请求的反向代理逻辑。
/// 该函数：
/// 1. 提取请求路径、主机和其他细节信息。
/// 2. 解析父路径和代理配置。
/// 3. 将请求转发到配置的代理服务器。
/// 4. 将代理服务器的响应返回给客户端。
///
/// # 参数
/// * `req_uri` - 入站请求的URI。
/// * `path` - 从请求中提取的可选路径参数。
/// * `req` - 入站的HTTP请求。
///
/// # 返回
/// 包含代理服务器响应或错误的 `RouteResult`。
#[axum::debug_handler]
pub async fn serve(
    req_uri: Uri,
    path: Option<Path<String>>,
    mut req: Request<Body>,
) -> RouteResult<impl IntoResponse> {
    let req_path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(req_path);

    let host = req
        .headers()
        .get("host") // 注意：host 是小写的
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    let scheme = req.uri().scheme_str().unwrap_or("http");
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
    tracing::debug!("Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    tracing::debug!("parent path: {:?}", parent_path);
    let proxy_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())?;
    tracing::debug!("proxy config: {:?}", proxy_config);

    // 确定代理目标 - 支持单一 proxy_pass 和 upstream 负载均衡
    let uri = if let Some(ref proxy_pass) = proxy_config.proxy_pass {
        format!("{proxy_pass}{path_query}")
    } else if let Some(ref upstream_name) = proxy_config.upstream {
        // 获取 upstream 配置
        let upstream = UPSTREAMS
            .get(upstream_name)
            .ok_or(RouteError::InternalError())?;

        // 加权轮询选择服务器
        let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
            .entry(upstream_name.clone())
            .or_insert_with(|| AtomicUsize::new(0));

        let current_counter = counter.fetch_add(1, Ordering::Relaxed);
        let total_weight: u32 = upstream.server.iter().map(|s| s.weight).sum();
        let mut current_weight = current_counter % total_weight as usize;

        let mut selected_index = 0;
        for (i, server) in upstream.server.iter().enumerate() {
            if current_weight < server.weight as usize {
                selected_index = i;
                break;
            }
            current_weight -= server.weight as usize;
        }

        let server = &upstream.server[selected_index];

        // 构建完整的代理 URI，确保正确的格式
        let server_addr =
            if server.server.starts_with("http://") || server.server.starts_with("https://") {
                server.server.clone()
            } else {
                format!("http://{}", server.server)
            };

        format!("{}{}", server_addr.trim_end_matches('/'), path_query)
    } else {
        return custom_page(proxy_config, req, true).await;
    };

    tracing::debug!("reverse proxy uri: {:?}", &uri);
    *req.uri_mut() = Uri::try_from(uri.clone()).map_err(|_| RouteError::InternalError())?;

    let timeout = proxy_config.proxy_timeout;

    // forward request headers
    let client = get_client();
    let mut forward_req = client
        .request(req.method().clone(), uri)
        .timeout(Duration::from_secs(timeout.into()));
    for (name, value) in req.headers().iter() {
        if !is_exclude_header(name) {
            forward_req = forward_req.header(name.clone(), value.clone());
        }
    }

    // forward request body
    let body = req.into_body();
    // 直接转发请求体，避免中间转换为字符串，提高性能
    let bytes = axum::body::to_bytes(body, 10 * 1024 * 1024)
        .await
        .map_err(|err| {
            tracing::error!("Failed to proxy request: {}", err);
            RouteError::InternalError()
        })?;
    forward_req = forward_req.body(bytes);

    // send reverse proxy request
    let reqwest_response = forward_req.send().await.map_err(|e| {
        tracing::error!("Failed to proxy request: {}", e);
        RouteError::BadRequest()
    })?;

    // response from reverse proxy server
    let mut response_builder = Response::builder().status(reqwest_response.status());
    copy_headers(
        reqwest_response.headers(),
        response_builder
            .headers_mut()
            .ok_or(RouteError::InternalError())?,
    );
    let res = response_builder
        .body(Body::from_stream(reqwest_response.bytes_stream()))
        .map_err(|e| {
            tracing::error!("Failed to proxy request: {}", e);
            RouteError::BadRequest()
        })?;

    Ok(res)
}

/// 检查给定的头部是否应该在反向代理中被排除转发。
/// 像 "host"、"connection" 等头部通常会被排除，以避免冲突或安全问题。
fn is_exclude_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host"
            | "connection"
            | "proxy-authenticate"
            | "upgrade"
            | "proxy-authorization"
            | "keep-alive"
            | "transfer-encoding"
            | "te"
    )
}

/// 将头部从一个 `HeaderMap` 复制到另一个，排除在 `is_exclude_header` 中指定的头部。
/// 这确保只转发相关的头部，避免冲突或安全问题。
fn copy_headers(from: &http::HeaderMap, to: &mut http::HeaderMap) {
    for (name, value) in from.iter() {
        if !is_exclude_header(name) {
            to.append(name.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn test_is_exclude_header() {
        // 测试应该排除的头部
        assert!(is_exclude_header(&http::header::HOST));
        assert!(is_exclude_header(&http::header::CONNECTION));
        assert!(is_exclude_header(&http::header::UPGRADE));
        assert!(is_exclude_header(&http::header::PROXY_AUTHENTICATE));
        assert!(is_exclude_header(&http::header::PROXY_AUTHORIZATION));
        assert!(is_exclude_header(&http::HeaderName::from_static(
            "keep-alive"
        )));
        assert!(is_exclude_header(&http::header::TRANSFER_ENCODING));
        assert!(is_exclude_header(&http::header::TE));

        // 测试不应该排除的头部
        assert!(!is_exclude_header(&http::header::USER_AGENT));
        assert!(!is_exclude_header(&http::header::CONTENT_TYPE));
        assert!(!is_exclude_header(&http::header::ACCEPT));
        assert!(!is_exclude_header(&http::header::AUTHORIZATION));
        assert!(!is_exclude_header(&http::header::COOKIE));
        assert!(!is_exclude_header(&http::header::REFERER));
    }

    #[test]
    fn test_copy_headers() {
        let mut from = http::HeaderMap::new();
        from.insert(http::header::HOST, HeaderValue::from_static("example.com"));
        from.insert(
            http::header::USER_AGENT,
            HeaderValue::from_static("test-agent"),
        );
        from.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain"),
        );
        from.insert(
            http::header::CONNECTION,
            HeaderValue::from_static("keep-alive"),
        );
        from.insert(http::header::ACCEPT, HeaderValue::from_static("*/*"));

        let mut to = http::HeaderMap::new();
        copy_headers(&from, &mut to);

        // 验证应该被排除的头部没有被复制
        assert!(!to.contains_key(http::header::HOST));
        assert!(!to.contains_key(http::header::CONNECTION));

        // 验证应该被复制的头部被正确复制
        assert_eq!(
            to.get(http::header::USER_AGENT),
            Some(&HeaderValue::from_static("test-agent"))
        );
        assert_eq!(
            to.get(http::header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("text/plain"))
        );
        assert_eq!(
            to.get(http::header::ACCEPT),
            Some(&HeaderValue::from_static("*/*"))
        );
    }
}
