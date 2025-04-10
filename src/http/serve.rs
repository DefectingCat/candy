use axum::{extract::Path, response::IntoResponse};
use http::Uri;
use tracing::debug;

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
    debug!("request: {:?} uri {}", path, parent_path);
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
