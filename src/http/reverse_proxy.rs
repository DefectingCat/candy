use std::time::Duration;

use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use http::{HeaderName, Uri};
use reqwest::Client;

use crate::{http::serve::resolve_parent_path, utils::parse_port_from_host};

use super::{
    HOSTS,
    error::{RouteError, RouteResult},
};

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
        // return custom_not_found!(host_route, request).await;
        return Err(RouteError::RouteNotFound());
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

fn copy_headers(from: &http::HeaderMap, to: &mut http::HeaderMap) {
    for (name, value) in from.iter() {
        if !is_exclude_header(name) {
            to.append(name.clone(), value.clone());
        }
    }
}
