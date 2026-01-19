use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use http::{Uri, header::LOCATION};

use crate::{
    http::{
        HOSTS,
        error::{RouteError, RouteResult},
        serve::resolve_parent_path,
    },
    utils::parse_port_from_host,
};

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
    let route_map = &HOSTS
        .get(&port)
        .ok_or(RouteError::BadRequest())
        .with_context(|| {
            format!("Hosts not found for port: {port}, host: {host}, scheme: {scheme}")
        })?
        .route_map;
    tracing::debug!("Redirect: Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    let route_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())
        .with_context(|| format!("route not found: {parent_path}"))?;

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
