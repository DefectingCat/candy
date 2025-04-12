use std::path::PathBuf;

use axum::{extract::Path, response::IntoResponse};
use http::Uri;
use tracing::debug;

use crate::{
    consts::HOST_INDEX,
    http::{ROUTE_MAP, error::RouteError},
};

use super::error::RouteResult;

/// Serve static files
/// If the request path matches a static file path, it will serve the file.
#[axum::debug_handler]
pub async fn serve(uri: Uri, path: Option<Path<String>>) -> RouteResult<impl IntoResponse> {
    // find parent path
    // if requested path is /doc
    // then params path is None
    // when Path is None, then use uri.path() as path

    // if request path is /doc/index.html
    // uri path is /doc/index.html
    // path is index.html
    // find parent path by path length
    // /doc/index.html
    // /doc/
    //      index.html

    let parent_path = match path {
        Some(ref path) => {
            let uri_path = uri.path();
            let parent_path = uri_path.get(0..uri_path.len() - path.len());
            parent_path.unwrap_or("/").to_string()
        }
        None => {
            let uri_path = uri.path().to_string();
            if uri_path.ends_with('/') {
                uri_path
            } else {
                format!("{}/", uri_path)
            }
        }
    };
    // parent_path is key in route map
    // which is `host_route.location`
    debug!("request: {:?} uri {:?}", path, parent_path);
    let route_map = ROUTE_MAP.read().await;
    // [TODO] custom error and not found page
    debug!("route map: {:?}", route_map);
    let Some(host_route) = route_map.get(&parent_path) else {
        return Err(RouteError::RouteNotFound());
    };
    debug!("route: {:?}", host_route);
    // after route found
    let Some(ref root) = host_route.root else {
        return Err(RouteError::RouteNotFound());
    };
    // try find index file first
    // build index filename as vec
    // ["./html/index.html", "./html/index.txt"]
    let path_arr = match path {
        Some(ref path) => {
            let path = PathBuf::from(path.to_string());
            let Some(index_name) = path.file_name() else {
                // [TODO] custom error and not found page
                return Err(RouteError::RouteNotFound());
            };
            vec![format!("{}/{}", root, index_name.to_string_lossy())]
        }
        None => {
            if host_route.index.is_empty() {
                HOST_INDEX
                    .iter()
                    .map(|s| format!("{}/{}", root, s))
                    .collect::<Vec<_>>()
            } else {
                host_route
                    .index
                    .iter()
                    .map(|s| format!("{}/{}", root, s))
                    .collect::<Vec<_>>()
            }
        }
    };
    debug!("request index file {:?}", path_arr);
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
