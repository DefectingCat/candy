use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use http::Uri;
use reqwest::Client;

use crate::utils::parse_port_from_host;

use super::{
    HOSTS,
    error::{RouteError, RouteResult},
};

#[axum::debug_handler]
pub async fn serve(
    req_uri: Uri,
    path: Option<Path<String>>,
    Host(host): Host,
    mut req: Request,
) -> RouteResult<impl IntoResponse> {
    let req_path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(req_path);

    // Resolve the parent path:
    // - If `path` is provided, extract the parent segment from the URI.
    // - If `path` is None, use the URI path directly (ensuring it ends with '/').
    /// Resolves the parent path from the URI and optional path segment.
    fn resolve_parent_path(uri: &Uri, path: Option<&Path<String>>) -> String {
        match path {
            Some(path) => {
                let uri_path = uri.path();
                // use path sub to this uri path
                // to find parent path that store in ROUTE_MAP
                // uri: /assets/css/styles.07713cb6.css, path: Some(Path("assets/css/styles.07713cb6.css")
                let parent_path = uri_path.get(0..uri_path.len() - path.len());
                parent_path.unwrap_or("/").to_string()
            }
            None => {
                // uri need end with /
                // because global ROUTE_MAP key is end with /
                // so we need add / to uri path to get correct Route
                let uri_path = uri.path().to_string();
                if uri_path.ends_with('/') {
                    uri_path
                } else {
                    format!("{uri_path}/")
                }
            }
        }
    }

    let scheme = req.uri().scheme_str().unwrap_or("http");
    let port = parse_port_from_host(&host, scheme).ok_or(RouteError::BadRequest())?;
    let route_map = &HOSTS.get(&port).ok_or(RouteError::BadRequest())?.route_map;
    tracing::debug!("Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    tracing::debug!("parent path: {:?}", parent_path);
    let proxy_pass = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())?;
    tracing::debug!("proxy pass: {:?}", proxy_pass);
    let Some(ref proxy_pass) = proxy_pass.proxy_pass else {
        // return custom_not_found!(host_route, request).await;
        return Err(RouteError::RouteNotFound());
    };
    let uri = format!("{proxy_pass}{path_query}");
    tracing::debug!("reverse proxy uri: {:?}", &uri);
    *req.uri_mut() = Uri::try_from(uri.clone()).map_err(|_| RouteError::InternalError())?;

    let client = Client::new();
    let reqwest_response = client.get(uri).send().await.map_err(|e| {
        tracing::error!("Failed to proxy request: {}", e);
        RouteError::BadRequest()
    })?;

    let mut response_builder = Response::builder().status(reqwest_response.status());
    *response_builder.headers_mut().unwrap() = reqwest_response.headers().clone();
    let res = response_builder
        .body(Body::from_stream(reqwest_response.bytes_stream()))
        // This unwrap is fine because the body is empty here
        .unwrap();

    Ok(res)
}
