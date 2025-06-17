use std::{path::PathBuf, str::FromStr, time::UNIX_EPOCH};

use anyhow::{Context, anyhow};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use dashmap::mapref::one::Ref;
use http::{
    HeaderValue, StatusCode, Uri,
    header::{CONTENT_TYPE, ETAG, IF_NONE_MATCH},
};
use mime_guess::from_path;
use tokio::fs::{self, File};
use tokio_util::io::ReaderStream;
use tracing::{debug, error};

use crate::{
    config::SettingRoute,
    consts::HOST_INDEX,
    http::{HOSTS, error::RouteError},
    utils::parse_port_from_host,
};

use super::error::RouteResult;

/// Macro to handle custom "not found" responses for a route.
///
/// When a requested file is not found, this macro:
/// 1. Checks if the `host_route` has a configured `not_found_page`.
/// 2. Attempts to serve the custom "not found" file (e.g., `404.html`).
/// 3. Falls back to `RouteNotFound` or `InternalError` if the file is missing or unreadable.
///
/// # Arguments
/// - `$host_route`: The route configuration containing `not_found_page` and `root` paths.
macro_rules! custom_not_found {
    ($host_route:expr, $request:expr) => {
        async {
            let page = $host_route
                .not_found_page
                .as_ref()
                .ok_or(RouteError::RouteNotFound())?;
            let root = $host_route
                .root
                .as_ref()
                .ok_or(RouteError::InternalError())?;
            let path = format!("{}/{}", root, page.page);
            let status = StatusCode::from_str(page.status.to_string().as_ref())
                .map_err(|_| RouteError::BadRequest())?;
            tracing::debug!("custom not found path: {:?}", path);
            match stream_file(path.into(), $request, Some(status)).await {
                Ok(res) => RouteResult::Ok(res),
                Err(e) => {
                    println!("Failed to stream file: {:?}", e);
                    RouteResult::Err(RouteError::InternalError())
                }
            }
        }
    };
}

/// Macro to handle custom "error" responses for a route.
///
/// When an internal server error occurs, this macro:
/// 1. Checks if the `host_route` has a configured `error_page`.
/// 2. Attempts to serve the custom "error" file (e.g., `500.html`).
/// 3. Falls back to `InternalError` if the file is missing or unreadable.
///
/// # Arguments
/// - `$host_route`: The route configuration containing `error_page` and `root` paths.
/// - `$request`: The HTTP request object.
macro_rules! custom_error_page {
    ($host_route:expr, $request:expr) => {
        async {
            let page = $host_route
                .error_page
                .as_ref()
                .ok_or(RouteError::InternalError())?;
            let root = $host_route
                .root
                .as_ref()
                .ok_or(RouteError::InternalError())?;
            let path = format!("{}/{}", root, page.page);
            let status = StatusCode::from_str(page.status.to_string().as_ref())
                .map_err(|_| RouteError::BadRequest())?;
            debug!("custom error path: {:?}", path);
            match stream_file(path.into(), $request, Some(status)).await {
                Ok(res) => RouteResult::Ok(res),
                Err(e) => {
                    println!("Failed to stream file: {:?}", e);
                    RouteResult::Err(RouteError::InternalError())
                }
            }
        }
    };
}

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
pub async fn serve(
    uri: Uri,
    path: Option<Path<String>>,
    Host(host): Host,
    request: Request,
) -> RouteResult<impl IntoResponse> {
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

    debug!(
        "Request - uri: {:?}, path: {:?}, request: {:?}",
        uri, path, request
    );

    let parent_path = resolve_parent_path(&uri, path.as_ref());
    // parent_path is key in route map
    // which is `host_route.location`
    let scheme = request.uri().scheme_str().unwrap_or("http");
    let port = parse_port_from_host(&host, scheme).ok_or(RouteError::BadRequest())?;
    let route_map = &HOSTS.get(&port).ok_or(RouteError::BadRequest())?.route_map;
    debug!("Route map entries: {:?}", route_map);
    let host_route = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())?;
    debug!("route: {:?}", host_route);
    // after route found
    // check static file root configuration
    // if root is None, then return InternalError
    let Some(ref root) = host_route.root else {
        return custom_not_found!(host_route, request).await;
    };
    // try find index file first
    // build index filename as vec
    // ["./html/index.html", "./html/index.txt"]
    // Build the list of candidate file paths to try:
    // - If `path` is provided, use it and check is file or not.
    // - If `path` is None, use the default index files (either from `host_route.index` or `HOST_INDEX`).
    let path_arr = if let Some(path) = path {
        #[allow(clippy::unnecessary_to_owned)]
        let path = path.to_string();
        if path.contains('.') {
            vec![format!("{}/{}", root, path)]
        } else {
            generate_default_index(&host_route, &format!("{root}/{path}"))
        }
    } else {
        generate_default_index(&host_route, root)
    };
    debug!("request index file {:?}", path_arr);
    // Try each candidate path in order:
    // - Return the first successfully streamed file.
    // - If all fail, return a `RouteNotFound` error.
    let mut path_exists = None;
    for path in path_arr {
        if fs::metadata(path.clone()).await.is_ok() {
            path_exists = Some(path);
            break;
        }
    }
    let Some(path_exists) = path_exists else {
        debug!("No valid file found in path candidates");
        return custom_not_found!(host_route, request).await;
    };
    match stream_file(path_exists.into(), request, None).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("Failed to stream file: {}", e);
            Err(RouteError::InternalError())
        }
    }
}

/// Generate default index files
/// if request path is not a file
/// this read config index field
/// and build with root: ["./html/index.html", "./html/index.txt"]
///
/// ## Arguments
/// - `host_route`: the host route config
/// - `root`: the root path
fn generate_default_index(host_route: &Ref<'_, String, SettingRoute>, root: &str) -> Vec<String> {
    let indices = if host_route.index.is_empty() {
        let host_iter = HOST_INDEX
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        host_iter.into_iter()
    } else {
        host_route.index.clone().into_iter()
    };
    indices.map(|s| format!("{root}/{s}")).collect()
}

/// Stream a file as an HTTP response.
///
/// # Arguments
/// - `path`: The filesystem path to the file.
///
/// # Returns
/// - `Ok(Response)`: If the file is successfully opened and streamed.
/// - `Err(anyhow::Error)`: If the file cannot be opened or read.
async fn stream_file(
    path: PathBuf,
    request: Request,
    status: Option<StatusCode>,
) -> RouteResult<impl IntoResponse> {
    let file = File::open(path.clone())
        .await
        .with_context(|| "open file failed")?;

    let path_str = path.to_str().ok_or(anyhow!("convert path to str failed"))?;
    let etag = calculate_etag(&file, path_str).await?;

    let mut response = Response::builder();
    let mut not_modified = false;
    // check request if-none-match
    if let Some(if_none_match) = request.headers().get(IF_NONE_MATCH)
        && if_none_match
            .to_str()
            .with_context(|| "parse if-none-match failed")?
            == etag
    {
        // let empty_stream = stream::empty::<u8>();
        // let body = Some(StreamBody::new(empty_stream));
        response = response.status(StatusCode::NOT_MODIFIED);
        not_modified = true;
    };

    let stream = if not_modified {
        let empty = File::open(PathBuf::from("/dev/null"))
            .await
            .with_context(|| "open /dev/null failed")?;
        ReaderStream::new(empty)
    } else {
        ReaderStream::new(file)
    };
    // let stream = stream.map(|res| res.map(Frame::data));
    // let body = StreamBody::new(stream);
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
    if let Some(status) = status {
        response = response.status(status);
    }
    let response = response
        .body(body)
        .with_context(|| "Failed to build HTTP response with body")?;
    Ok(response)
}

pub async fn calculate_etag(file: &File, path: &str) -> anyhow::Result<String> {
    // calculate file metadata as etag
    let metadata = file
        .metadata()
        .await
        .with_context(|| "get file metadata failed")?;
    let created_timestamp = metadata
        .created()
        .with_context(|| "get file created failed")?
        .duration_since(UNIX_EPOCH)
        .with_context(|| "calculate unix timestamp failed")?
        .as_secs();
    let modified_timestamp = metadata
        .modified()
        .with_context(|| "get file created failed")?
        .duration_since(UNIX_EPOCH)
        .with_context(|| "calculate unix timestamp failed")?
        .as_secs();
    // file path - created - modified - len
    let etag = format!(
        "{}-{}-{}-{}",
        path,
        created_timestamp,
        modified_timestamp,
        metadata.len()
    );
    let etag = format!("W/\"{:?}\"", md5::compute(etag));
    debug!("file {:?} etag: {:?}", path, etag);
    Ok(etag)
}

// Resolve the parent path:
// - If `path` is provided, extract the parent segment from the URI.
// - If `path` is None, use the URI path directly (ensuring it ends with '/').
/// Resolves the parent path from the URI and optional path segment.
pub fn resolve_parent_path(uri: &Uri, path: Option<&Path<String>>) -> String {
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
            // uri needs end with /
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
