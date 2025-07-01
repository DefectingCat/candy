use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, Request},
    response::IntoResponse,
};
use axum_extra::extract::Host;
use http::Uri;
use tracing::error;

use crate::{
    http::{HOSTS, error::RouteError, serve::resolve_parent_path},
    utils::parse_port_from_host,
};

use super::error::RouteResult;

pub async fn lua(
    req_uri: Uri,
    path: Option<Path<String>>,
    Host(host): Host,
    req: Request<Body>,
) -> RouteResult<impl IntoResponse> {
    let req_path = req.uri().path();

    let scheme = req.uri().scheme_str().unwrap_or("http");
    let port = parse_port_from_host(&host, scheme).ok_or(RouteError::BadRequest())?;
    let route_map = &HOSTS
        .get(&port)
        .ok_or(RouteError::BadRequest())
        .with_context(|| {
            format!("Hosts not found for port: {port}, host: {host}, scheme: {scheme}")
        })?
        .route_map;
    tracing::debug!("Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    let route_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())
        .with_context(|| format!("route not found: {parent_path}"))?;
    let lua_script = route_config
        .lua_script
        .as_ref()
        .ok_or(RouteError::InternalError())
        .with_context(|| "lua script not found")?;
    error!("Lua script: {lua_script}");
    Ok(())
}
