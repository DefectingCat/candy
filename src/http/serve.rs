use std::path::PathBuf;

use axum::{extract::Path, response::IntoResponse};
use http::Uri;
use tracing::debug;

use crate::http::{ROUTE_MAP, error::RouteError};

use super::error::RouteResult;

/// Serve static files
/// If the request path matches a static file path, it will serve the file.
#[axum::debug_handler]
pub async fn serve(uri: Uri, Path(path): Path<String>) -> RouteResult<impl IntoResponse> {
    // find parent path
    // if request path is /doc/index.html
    // uri path is /doc/index.html
    // path is index.html
    // find parent path by path length
    // /doc/index.html
    // /doc/
    //      index.html
    let uri_path = uri.path();
    let parent_path = uri_path.get(0..uri_path.len() - path.len());
    let parent_path = parent_path.unwrap_or("/");
    // parent_path is key in route map
    // which is `host_route.location`
    debug!("request: {:?} uri {}", path, parent_path);
    let route_map = ROUTE_MAP.read().await;
    // [TODO] custom error and not found page
    let Some(host_route) = route_map.get(parent_path) else {
        return Err(RouteError::RouteNotFound());
    };
    debug!("route: {:?}", host_route);
    let path = PathBuf::from(path);
    let Some(index_name) = path.file_name() else {
        return Err(RouteError::RouteNotFound());
    };
    // after route found
    debug!("request index file {:?}", index_name);
    // try find index file first
    // let host_route = app
    //     .host_route
    //     .get(&request.uri().path().to_string())
    //     .unwrap();
    // let has_html = host_route.index.iter().any(|s| s == ".html");
    // let Some(root) = host_route.root.as_ref() else {
    //     return Err(RouteError::Any(anyhow::anyhow!("root field not found")));
    // };
    // if has_html {
    //     let service = ServeDir::new(root);
    //     let res = service.oneshot(request).await?;
    //     return Ok(res);
    // } else {
    //     return Err(RouteError::Any(anyhow::anyhow!("root field not found")));
    // }
    Ok(())
}
