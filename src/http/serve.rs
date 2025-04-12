use std::path::PathBuf;

use axum::{
    extract::Path,
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use http::{HeaderValue, Uri, header::CONTENT_TYPE};
use http_body_util::StreamBody;
use hyper::body::Frame;
use mime_guess::from_path;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::debug;

use crate::{
    consts::HOST_INDEX,
    http::{ROUTE_MAP, error::RouteError},
};

use super::error::RouteResult;

/// Serve static files.
///
/// This function handles requests for static files by:
/// 1. Resolving the parent path from the URI or provided path.
/// 2. Looking up the route in `ROUTE_MAP` to find the root directory.
/// 3. Attempting to serve the requested file or a default index file.
///
/// # Arguments
/// - `uri`: The request URI, used to extract the full path.
/// - `path`: Optional path segment provided by the router.
///
/// # Returns
/// - `Ok(Response)`: If the file is found and successfully streamed.
/// - `Err(RouteError)`: If the route or file is not found.
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

    debug!("Request - uri: {:?}, path: {:?}", uri, path);
    // Resolve the parent path:
    // - If `path` is provided, extract the parent segment from the URI.
    // - If `path` is None, use the URI path directly (ensuring it ends with '/').
    /// Resolves the parent path from the URI and optional path segment.
    fn resolve_parent_path(uri: &Uri, path: Option<&Path<String>>) -> String {
        match path {
            Some(path) => {
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
        }
    }

    let parent_path = resolve_parent_path(&uri, path.as_ref());
    // parent_path is key in route map
    // which is `host_route.location`
    debug!(
        "Request - path: {:?}, parent_path: {:?}, uri: {:?}",
        path, parent_path, uri
    );
    let route_map = ROUTE_MAP.read().await;
    // [TODO] custom error and not found page
    debug!("Route map entries: {:?}", route_map.keys());
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
    // Build the list of candidate file paths to try:
    // - If `path` is provided, use it directly.
    // - If `path` is None, use the default index files (either from `host_route.index` or `HOST_INDEX`).
    let path_arr = if let Some(path) = path {
        #[allow(clippy::unnecessary_to_owned)]
        let path = path.to_string();
        vec![format!("{}/{}", root, path)]
    } else {
        let indices = if host_route.index.is_empty() {
            let host_iter = HOST_INDEX
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            host_iter.into_iter()
        } else {
            host_route.index.clone().into_iter()
        };
        indices.map(|s| format!("{}/{}", root, s)).collect()
    };
    debug!("request index file {:?}", path_arr);
    // Try each candidate path in order:
    // - Return the first successfully streamed file.
    // - If all fail, return a `RouteNotFound` error.
    for path in path_arr {
        match stream_file(path.into()).await {
            Ok(res) => return Ok(res),
            Err(e) => debug!("Failed to stream file: {}", e),
        }
    }
    debug!("No valid file found in path candidates");
    Err(RouteError::RouteNotFound())
}

/// Stream a file as an HTTP response.
///
/// # Arguments
/// - `path`: The filesystem path to the file.
///
/// # Returns
/// - `Ok(Response)`: If the file is successfully opened and streamed.
/// - `Err(anyhow::Error)`: If the file cannot be opened or read.
async fn stream_file(path: PathBuf) -> anyhow::Result<impl IntoResponse> {
    let file = File::open(path.clone()).await?;
    let stream = ReaderStream::new(file);
    let stream = stream.map(|res| res.map(Frame::data));
    let body = StreamBody::new(stream);

    let mime = from_path(path).first_or_octet_stream();
    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(mime.as_ref())?);

    Ok(response)
}
