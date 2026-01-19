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
    debug!("Redirect: Route map entries: {:?}", route_map);

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
