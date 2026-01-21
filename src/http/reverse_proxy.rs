use std::time::Duration;

use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use http::{
    HeaderName, HeaderValue, Uri,
    header::{CONTENT_TYPE, ETAG},
};
use mime_guess::from_path;
use reqwest::Client;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use super::{
    HOSTS,
    error::{RouteError, RouteResult},
};
use crate::http::serve::{check_if_none_match, empty_stream};
use crate::{
    config::SettingRoute,
    http::serve::{calculate_etag, resolve_parent_path},
    utils::parse_port_from_host,
};

/// 处理自定义错误页面（如404、500等）的请求
///
/// 该函数根据配置信息加载自定义错误页面文件，并根据HTTP缓存机制
/// 决定是返回完整内容还是304 Not Modified状态码。
///
/// # 参数
/// - `host_config`: 主机路由配置，包含错误页面路径和根目录信息
/// - `request`: 原始HTTP请求
/// - `is_error_page`: 是否为错误页面（true: 错误页，false: 404页）
///
/// # 返回
/// - `Ok(Response)`: 成功时返回HTTP响应
/// - `Err(RouteError)`: 失败时返回路由错误
pub async fn handle_custom_page(
    host_config: dashmap::mapref::one::Ref<'_, String, SettingRoute>,
    request: Request<Body>,
    is_error_page: bool,
) -> RouteResult<Response<Body>> {
    // 根据请求类型选择相应的页面配置
    let page = if is_error_page {
        host_config
            .error_page
            .as_ref()
            .ok_or(RouteError::RouteNotFound())?
    } else {
        host_config
            .not_found_page
            .as_ref()
            .ok_or(RouteError::RouteNotFound())?
    };

    // 获取站点根目录配置
    let root = host_config
        .root
        .as_ref()
        .ok_or(RouteError::InternalError())?;

    // 构建完整文件路径
    let path = format!("{}/{}", root, page.page);
    tracing::debug!("custom not found path: {:?}", path);

    // 打开文件并计算ETag用于缓存验证
    let file = File::open(path.clone())
        .await
        .with_context(|| "open file failed")?;

    let etag = calculate_etag(&file, path.as_str()).await?;
    let response = Response::builder();
    let (mut response, not_modified) = check_if_none_match(request, &etag, response);

    // 准备响应主体
    let stream = if not_modified {
        // 304 响应返回空内容
        empty_stream().await?
    } else {
        // 正常响应返回文件内容
        ReaderStream::new(file)
    };
    let body = Body::from_stream(stream);

    // 设置响应头：内容类型和ETag
    let mime = from_path(path).first_or_octet_stream();
    response
        .headers_mut()
        .with_context(|| "insert header failed")?
        .insert(
            CONTENT_TYPE,
            HeaderValue::from_str(mime.as_ref()).with_context(|| "insert header failed")?,
        );
    response
        .headers_mut()
        .with_context(|| "insert header failed")?
        .insert(
            ETAG,
            HeaderValue::from_str(&etag).with_context(|| "insert header failed")?,
        );

    // 构建最终响应
    let response = response
        .body(body)
        .with_context(|| "Failed to build HTTP response with body")?;
    Ok(response)
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
        let port_config = HOSTS
            .get(&port)
            .ok_or(RouteError::BadRequest())
            .with_context(|| {
                format!("Hosts not found for port: {port}, host: {host}, scheme: {scheme}")
            })?;

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

        host_config
            .ok_or(RouteError::BadRequest())
            .with_context(|| format!("Host configuration not found for domain: {domain}"))?
    };

    let route_map = &host_config.route_map;
    tracing::debug!("Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    tracing::debug!("parent path: {:?}", parent_path);
    let proxy_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())
        .with_context(|| format!("route not found: {parent_path}"))?;
    tracing::debug!("proxy pass: {:?}", proxy_config);
    let Some(ref proxy_pass) = proxy_config.proxy_pass else {
        return handle_custom_page(proxy_config, req, true).await;
    };
    let uri = format!("{proxy_pass}{path_query}");
    tracing::debug!("reverse proxy uri: {:?}", &uri);
    *req.uri_mut() = Uri::try_from(uri.clone())
        .map_err(|_| RouteError::InternalError())
        .with_context(|| format!("uri not found: {uri}"))?;

    let timeout = proxy_config.proxy_timeout;

    // forward request headers
    let client = Client::new();
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
    let bytes = axum::body::to_bytes(body, 2048).await.map_err(|err| {
        tracing::error!("Failed to proxy request: {}", err);
        RouteError::InternalError()
    })?;
    let body_str = String::from_utf8(bytes.to_vec()).map_err(|err| {
        tracing::error!("Failed to proxy request: {}", err);
        RouteError::InternalError()
    })?;
    forward_req = forward_req.body(body_str);

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
            .ok_or(RouteError::InternalError())
            .with_context(|| "headers not found")?,
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
