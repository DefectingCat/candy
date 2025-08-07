use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
    str::FromStr,
    time::UNIX_EPOCH,
};

use anyhow::{Context, anyhow};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use dashmap::mapref::one::Ref;
use http::response::Builder;
use http::{
    HeaderMap, HeaderValue, StatusCode, Uri,
    header::{CONTENT_TYPE, ETAG, IF_NONE_MATCH, LOCATION},
};
use mime_guess::from_path;
use tokio::fs::{self, File};
use tokio_util::io::ReaderStream;
use tracing::{debug, error, warn};

use crate::{
    config::SettingRoute,
    consts::HOST_INDEX,
    http::{HOSTS, error::RouteError},
    utils::parse_port_from_host,
};

use super::error::RouteResult;

/// 处理自定义页面请求（如404错误页或自定义错误页面）
///
/// 此函数根据请求类型（错误页或404页）加载相应的自定义页面，
/// 构建完整文件路径并尝试流式传输文件内容作为HTTP响应。
///
/// # 参数
/// - `host_route`: 主机路由配置引用，包含页面位置和根目录信息
/// - `request`: 原始HTTP请求
/// - `is_error_page`: 是否为错误页面（true: 错误页，false: 404页）
///
/// # 返回
/// - `RouteResult<Response>`: 成功时返回HTTP响应，失败时返回路由错误
async fn custom_page(
    host_route: Ref<'_, String, SettingRoute>,
    request: Request<Body>,
    is_error_page: bool,
) -> RouteResult<Response<Body>> {
    let page = if is_error_page {
        host_route
            .error_page
            .as_ref()
            .ok_or(RouteError::RouteNotFound())
            .with_context(|| "error page not found")?
    } else {
        host_route
            .not_found_page
            .as_ref()
            .ok_or(RouteError::RouteNotFound())
            .with_context(|| "not found page not found")?
    };

    let root = host_route
        .root
        .as_ref()
        .ok_or(RouteError::InternalError())
        .with_context(|| "root not found")?;

    let path = format!("{}/{}", root, page.page);

    let status = StatusCode::from_str(page.status.to_string().as_ref())
        .map_err(|_| RouteError::BadRequest())
        .with_context(|| format!("status code not found: {}", page.status))?;

    debug!("custom not found path: {:?}", path);

    match stream_file(path.into(), request, Some(status)).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("Failed to stream file: {:?}", e);
            Err(RouteError::InternalError())
        }
    }
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

    // parent_path is key in route map
    // which is `host_route.location`
    let parent_path = resolve_parent_path(&uri, path.as_ref());
    let scheme = request.uri().scheme_str().unwrap_or("http");
    // port is key in route_map
    // which is `host_route.port`, used to find current host configuration
    let port = parse_port_from_host(&host, scheme).ok_or(RouteError::BadRequest())?;
    // route_map can be found by port
    // current host configruation
    let route_map = &HOSTS
        .get(&port)
        .ok_or(RouteError::BadRequest())
        .with_context(|| {
            format!("Hosts not found for port: {port}, host: {host}, scheme: {scheme}")
        })?
        .route_map;
    debug!("Route map entries: {:?}", route_map);
    // host_route can be found by parent_path
    // current route configuration
    let host_route = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())
        .with_context(|| format!("route not found: {parent_path}"))?;
    debug!("route: {:?}", host_route);
    // after route found
    // check static file root configuration
    // if root is None, then return InternalError
    let Some(ref root) = host_route.root else {
        return custom_page(host_route, request, true).await;
    };
    // try find index file first
    // build index filename as vec
    // ["./html/index.html", "./html/index.txt"]
    // Build the list of candidate file paths to try:
    // - If `path` is provided, use it and check is file or not.
    // - If `path` is None, use the default index files (either from `host_route.index` or `HOST_INDEX`).
    // path_arr 是包含默认索引文件的数组
    // req_path 是请求的路径
    let (req_path, path_arr) = if let Some(path) = path {
        #[allow(clippy::unnecessary_to_owned)]
        let path = path.to_string();
        if path.contains('.') {
            (root.into(), vec![format!("{}/{}", root, path)])
        } else {
            generate_default_index(&host_route, &format!("{root}/{path}"))
        }
    } else {
        generate_default_index(&host_route, root)
    };
    debug!("request index file {:?}", path_arr);
    debug!("req_path: {:?}", req_path);

    // 检查是否开启自动生成目录索引
    let uri_path = uri.path();
    debug!("uri_path: {:?}", uri_path);
    let uri_path_vec = uri_path.split('/').collect::<Vec<&str>>();
    let uri_path_last = uri_path_vec.last();
    debug!("uri_path_last: {:?}", uri_path_last);
    let uri_path_last = uri_path_last.unwrap_or(&"");
    if host_route.auto_index && !uri_path_last.contains('.') {
        // HTML 中的标题路径，需要移除掉配置文件中的 root = "./html" 字段
        let host_root = if let Some(root) = &host_route.root {
            root
        } else {
            return custom_page(host_route, request, false).await;
        };
        let req_path_str = req_path.to_string_lossy();
        debug!("req_path_str: {:?}", req_path_str);
        let host_root = &req_path_str.strip_prefix(host_root).unwrap_or(host_root);
        let list = list_dir(&req_path_str, &req_path).await?;
        let list_html = render_list_html(host_root, list);
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
        return Ok((headers, list_html).into_response());
    }

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
    debug!("path_exists: {:?}", path_exists);
    // 检查路径是否存在
    // 不存时，检查是否开启自动生成目录索引
    let path_exists = match path_exists {
        Some(path_exists) => path_exists,
        None => {
            let uri_path = uri.path();
            debug!("uri_path: {:?}", uri_path);
            // 如果请求路径不是文件且不以 / 结尾，则返回 301 Moved Permanently 状态码
            if !uri_path.ends_with('/') && !uri_path.contains('.') {
                let mut response = Response::builder();
                let stream = empty_stream().await?;
                let body = Body::from_stream(stream);
                response
                    .headers_mut()
                    .with_context(|| "insert header failed")?
                    .insert(
                        LOCATION,
                        HeaderValue::from_str(format!("{uri_path}/").as_str())
                            .with_context(|| "insert header failed")?,
                    );
                response = response.status(StatusCode::MOVED_PERMANENTLY);
                let response = response
                    .body(body)
                    .with_context(|| "Failed to build HTTP response with body")?;
                return Ok(response);
            }
            // 生成自动目录索引
            return if host_route.auto_index {
                // HTML 中的标题路径，需要移除掉配置文件中的 root = "./html" 字段
                let host_root = if let Some(root) = &host_route.root {
                    root
                } else {
                    return custom_page(host_route, request, false).await;
                };
                let req_path_str = req_path.to_string_lossy();
                debug!("req_path_str: {:?}", req_path_str);
                let host_root = &req_path_str.strip_prefix(host_root).unwrap_or(host_root);
                let list = list_dir(&req_path_str, &req_path).await?;
                let list_html = render_list_html(host_root, list);
                let mut headers = HeaderMap::new();
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
                Ok((headers, list_html).into_response())
            } else {
                debug!("No valid file found in path candidates");
                custom_page(host_route, request, false).await
            };
        }
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
///
/// ## Returns
/// - PathBuf: 客户端访问的路径
/// - Vec<String>: 包含默认索引文件名的数组
fn generate_default_index(
    host_route: &Ref<'_, String, SettingRoute>,
    root: &str,
) -> (PathBuf, Vec<String>) {
    let indices = if host_route.index.is_empty() {
        // use default index files
        let host_iter = HOST_INDEX
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        host_iter.into_iter()
    } else {
        host_route.index.clone().into_iter()
    };
    // indices 就是 host_route.index 的中配置的 index 文件名
    (
        root.into(),
        indices.map(|s| format!("{root}/{s}")).collect(),
    )
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
) -> RouteResult<Response<Body>> {
    let file = File::open(path.clone())
        .await
        .with_context(|| "open file failed")?;

    let path_str = path.to_str().ok_or(anyhow!("convert path to str failed"))?;
    let etag = calculate_etag(&file, path_str).await?;

    let response = Response::builder();
    let (mut response, not_modified) = check_if_none_match(request, &etag, response);

    let stream = if not_modified {
        empty_stream().await?
    } else {
        ReaderStream::new(file)
    };
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

pub fn check_if_none_match(request: Request, etag: &String, response: Builder) -> (Builder, bool) {
    let mut not_modified = false;
    // check request if-none-match
    if let Some(if_none_match) = request.headers().get(IF_NONE_MATCH) {
        if let Ok(if_none_match_str) = if_none_match.to_str() {
            if if_none_match_str == etag {
                not_modified = true;
                return (response.status(StatusCode::NOT_MODIFIED), not_modified);
            }
        }
    }
    (response, not_modified)
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

/// 生成一个 HTML 目录列表页面，展示指定目录中的文件和子目录。
///
/// 该函数将一个 `DirList` 结构体的向量转换为 HTML 表格格式，
/// 每个条目包含名称（带链接）、最后修改时间和大小信息。
///
/// # 参数
/// * `root_path` - 目录路径 显示在 HTML 中的根目录
/// * `list` - 包含目录项信息的 `DirList` 结构体向量
///
/// # 返回值
/// 格式化后的 HTML 字符串，可直接作为 HTTP 响应返回
///
/// # 示例
/// ```rust
/// let dir_entries = vec![
///     DirList {
///         path: PathBuf::from("/home/user/docs"),
///         name: "documents".to_string(),
///         last_modified: "2023-05-15 14:30".to_string(),
///         size: "4.2K".to_string(),
///         is_dir: true
///     },
///     // 更多条目...
/// ];
///
/// let html_output = render_list_html(dir_entries);
/// println!("{}", html_output);
/// ```
fn render_list_html(root_path: &str, list: Vec<DirList>) -> String {
    debug!(
        "render list html list: {:?} root_path: {:?}",
        list, root_path
    );
    // 先生成目标目录下所有文件的行
    let body_rows = list
        .iter()
        .map(|dist| {
            if dist.is_dir {
                format!(
                    r#"<tr><td><a href="{}">{}/</a></td><td>{}</td><td>{}</td></tr>"#,
                    dist.path, dist.name, dist.last_modified, dist.size,
                )
            } else {
                format!(
                    r#"<tr><td><a href="{}">{}</a></td><td>{}</td><td>{}</td></tr>"#,
                    dist.path, dist.name, dist.last_modified, dist.size,
                )
            }
        })
        .collect::<Vec<String>>()
        .join("");

    let list_html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Index of {root_path}</title>
    <style>
        body {{
            font-family: Arial, sans-serif;
            margin: 20px;
            background-color: #ffffff;
            color: #000000;
        }}
        h1 {{
            font-size: 1.5em;
            margin-bottom: 20px;
            text-align: left;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            border: 1px solid #dddddd;
        }}
        th, td {{
            padding: 8px 12px;
            text-align: left;
            border-bottom: 1px solid #dddddd;
        }}
        th {{
            background-color: #f0f0f0;
            font-weight: bold;
        }}
        tr:nth-child(even) {{
            background-color: #f9f9f9;
        }}
        tr:hover {{
            background-color: #f0f0f0;
        }}
        .dir {{
            color: #0066cc;
            font-weight: bold;
        }}
        .file {{
            color: #000000;
        }}
        a {{
            text-decoration: none;
            color: inherit;
        }}
        a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <h1>Index of {root_path}</h1>
    <table>
        <tr>
            <th>Name</th>
            <th>Last Modified</th>
            <th>Size</th>
        </tr>
        <tbody id="directory-content">
            {body_rows}
        </tbody>
    </table>
</body>
</html>
    "#,
    );
    list_html
}

const KB: u64 = 1024;
const KB1: u64 = KB + 1;
const MB: u64 = 1024 * 1024;
const MB1: u64 = MB + 1;
const GB: u64 = 1024 * 1024 * 1024;
const GB1: u64 = GB + 1;
const TB: u64 = 1024 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct ByteUnit(u64);

impl Display for ByteUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0..=KB => write!(f, "{} B", self.0),
            KB1..=MB => write!(f, "{:.2} KB", self.0 as f64 / 1024.0),
            MB1..=GB => write!(f, "{:.2} MB", self.0 as f64 / 1024.0 / 1024.0),
            GB1..=TB => write!(f, "{:.2} TB", self.0 as f64 / 1024.0 / 1024.0 / 1024.0),
            _ => write!(f, "{} B", self.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DirList {
    pub name: String,          // 文件或目录名称
    pub path: String,          // 文件或目录的完整路径
    pub is_dir: bool,          // 是否为目录
    pub size: ByteUnit,        // 文件大小（字节）
    pub last_modified: String, // 最后修改时间的字符串表示
}

/// 异步列出指定目录下的所有文件和子目录信息
///
/// # 参数
/// * `path` - 要列出内容的目录路径
///
/// # 返回
/// 成功时返回包含 `DirList` 结构的向量，失败时返回错误
///
/// # 错误
/// 可能返回与文件系统操作相关的错误，如目录不存在、权限不足等
async fn list_dir(host_root_str: &str, path: &PathBuf) -> anyhow::Result<Vec<DirList>> {
    use chrono::{Local, TimeZone};
    use std::time::UNIX_EPOCH;

    let mut list = vec![];
    // 异步读取目录条目
    let mut entries = fs::read_dir(path)
        .await
        .with_context(|| format!("无法读取目录: {}", path.display()))?;

    debug!("list dir path: {:?}", path);

    let mut tasks = vec![];
    // 遍历目录中的每个条目
    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("读取目录条目失败: {}", path.display()))?
    {
        #[cfg(unix)]
        let host_root_str = if host_root_str.ends_with('/') {
            host_root_str.to_string()
        } else {
            format!("{host_root_str}/")
        };
        // Windows 与 Unix 系统下的路劲处理方式不同
        #[cfg(windows)]
        let host_root_str = if host_root_str.ends_with('/') {
            host_root_str
                .strip_suffix('/')
                .ok_or(anyhow!("list_dir: strip host_root_str suffix failed"))?
                .to_string()
        } else {
            host_root_str.to_string()
        };
        // 为每个条目创建异步任务，并行获取元数据
        let task = tokio::task::spawn(async move {
            // 获取文件元数据
            let metadata = entry
                .metadata()
                .await
                .with_context(|| "获取文件元数据失败")?;

            // 获取并格式化最后修改时间
            let last_modified = metadata
                .modified()
                .with_context(|| "获取文件修改时间失败")?;
            let last_modified = last_modified
                .duration_since(UNIX_EPOCH)
                .with_context(|| "计算 Unix 时间戳失败")?;

            // 转换为本地时间，处理可能的歧义情况
            let datetime = match Local
                .timestamp_opt(last_modified.as_secs() as i64, last_modified.subsec_nanos())
            {
                chrono::LocalResult::Ambiguous(earlier, later) => {
                    warn!("发现歧义时间: {} 和 {}", earlier, later);
                    earlier
                }
                chrono::offset::LocalResult::Single(single) => {
                    // warn!("发现歧义时间: {}", single);
                    single
                }
                chrono::offset::LocalResult::None => {
                    error!("无法解析时间时使用当前时间");
                    Local::now()
                }
            };
            let last_modified = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

            // 收集其他元数据
            let size = ByteUnit(metadata.len());
            let is_dir = metadata.is_dir();
            let name = entry.file_name().to_string_lossy().to_string();

            let path = entry
                .path()
                .to_string_lossy()
                .strip_prefix(&host_root_str)
                .ok_or(anyhow!("list_dir: strip prefix failed"))?
                .to_string();
            let path = if is_dir {
                format!("./{path}/")
            } else {
                format!("./{path}")
            };
            // 创建并返回目录条目信息
            let dir = DirList {
                name,
                path,
                is_dir,
                size,
                last_modified,
            };
            anyhow::Ok(dir)
        });
        tasks.push(task);
    }

    // 等待所有异步任务完成并收集结果
    for task in tasks {
        list.push(task.await??);
    }

    Ok(list)
}

/// 创建一个空数据流，用于返回空响应或占位数据
///
/// 在不同操作系统上，会自动选择对应的空设备文件：
/// - Windows: NUL
/// - Unix/Linux: /dev/null
///
/// 返回一个异步流，内容为一个空文件的数据流
///
/// # 错误处理
/// 如果无法打开空设备文件，会返回带有上下文信息的错误
pub async fn empty_stream() -> anyhow::Result<ReaderStream<File>> {
    #[cfg(windows)]
    let null = PathBuf::from("NUL");
    #[cfg(not(windows))]
    let null = PathBuf::from("/dev/null");
    let empty = File::open(null)
        .await
        .with_context(|| "open /dev/null failed")?;
    Ok(ReaderStream::new(empty))
}
