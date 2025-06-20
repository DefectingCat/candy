use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use http::{
    HeaderName, HeaderValue, StatusCode, Uri,
    header::{CONTENT_TYPE, ETAG, IF_NONE_MATCH},
};
use mime_guess::from_path;
use reqwest::Client;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    http::serve::{calculate_etag, resolve_parent_path},
    utils::parse_port_from_host,
};

use super::{
    HOSTS,
    error::{RouteError, RouteResult},
};

macro_rules! custom_not_found {
    ($host_config:expr, $request:expr, $is_error_page:expr) => {{
        let page = if $is_error_page {
            $host_config
                .error_page
                .as_ref()
                .ok_or(RouteError::RouteNotFound())?
        } else {
            $host_config
                .not_found_page
                .as_ref()
                .ok_or(RouteError::RouteNotFound())?
        };
        let root = $host_config
            .root
            .as_ref()
            .ok_or(RouteError::InternalError())?;
        let path = format!("{}/{}", root, page.page);
        tracing::debug!("custom not found path: {:?}", path);

        let file = File::open(path.clone())
            .await
            .with_context(|| "open file failed")?;

        let etag = calculate_etag(&file, path.as_str()).await?;
        let mut response = Response::builder();
        let mut not_modified = false;

        // if let Some(if_none_match) = $request.headers().get(IF_NONE_MATCH)
        //     && if_none_match
        //         .to_str()
        //         .with_context(|| "parse if-none-match failed")?
        //         == etag
        // {
        //     response = response.status(StatusCode::NOT_MODIFIED);
        //     not_modified = true;
        // };
        if let Some(if_none_match) = $request.headers().get(IF_NONE_MATCH) {
            if let Ok(if_none_match_str) = if_none_match.to_str() {
                if if_none_match_str == etag {
                    response = response.status(StatusCode::NOT_MODIFIED);
                    not_modified = true;
                }
            }
        }

        let stream = if not_modified {
            let empty = File::open(PathBuf::from("/dev/null"))
                .await
                .with_context(|| "open /dev/null failed")?;
            ReaderStream::new(empty)
        } else {
            ReaderStream::new(file)
        };
        let body = Body::from_stream(stream);

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

        let response = response
            .body(body)
            .with_context(|| "Failed to build HTTP response with body")?;
        Ok(response)
    }};
}

/// Handles the reverse proxy logic for incoming requests.
/// This function:
/// 1. Extracts the request path, host, and other details.
/// 2. Resolves the parent path and proxy configuration.
/// 3. Forwards the request to the configured proxy server.
/// 4. Returns the response from the proxy server to the client.
///
/// # Arguments
/// * `req_uri` - The URI of the incoming request.
/// * `path` - Optional path parameter extracted from the request.
/// * `host` - The host header from the request.
/// * `req` - The incoming HTTP request.
///
/// # Returns
/// A `RouteResult` containing the response from the proxy server or an error.
#[axum::debug_handler]
pub async fn serve(
    req_uri: Uri,
    path: Option<Path<String>>,
    Host(host): Host,
    mut req: Request<Body>,
) -> RouteResult<impl IntoResponse> {
    let req_path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(req_path);

    let scheme = req.uri().scheme_str().unwrap_or("http");
    let port = parse_port_from_host(&host, scheme).ok_or(RouteError::BadRequest())?;
    let route_map = &HOSTS.get(&port).ok_or(RouteError::BadRequest())?.route_map;
    tracing::debug!("Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    tracing::debug!("parent path: {:?}", parent_path);
    let proxy_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())?;
    tracing::debug!("proxy pass: {:?}", proxy_config);
    let Some(ref proxy_pass) = proxy_config.proxy_pass else {
        return custom_not_found!(proxy_config, req, true);
    };
    let uri = format!("{proxy_pass}{path_query}");
    tracing::debug!("reverse proxy uri: {:?}", &uri);
    *req.uri_mut() = Uri::try_from(uri.clone()).map_err(|_| RouteError::InternalError())?;

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
    // TODO: set body size limit
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

/// Checks if a given header should be excluded from being forwarded in the reverse proxy.
/// Headers like "host", "connection", etc., are typically excluded to avoid conflicts or security issues.
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

/// Copies headers from one `HeaderMap` to another, excluding headers specified in `is_exclude_header`.
/// This ensures only relevant headers are forwarded, avoiding conflicts or security issues.
fn copy_headers(from: &http::HeaderMap, to: &mut http::HeaderMap) {
    for (name, value) in from.iter() {
        if !is_exclude_header(name) {
            to.append(name.clone(), value.clone());
        }
    }
}
