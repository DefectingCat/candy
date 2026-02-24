use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
    time::UNIX_EPOCH,
};

use anyhow::{Context, anyhow};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
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
    http::{HOSTS, error::RouteError},
    utils::parse_port_from_host,
};

use super::error::RouteResult;

/// 处理自定义页面请求（如404错误页面或自定义错误页面）
/// 该函数根据请求类型（错误页面或404页面）加载对应的自定义页面，
/// 构造完整的文件路径，并尝试将文件内容流式传输为HTTP响应。
/// - `host_route`: 包含页面位置和根目录信息的主机路由配置引用
/// - `request`: 原始HTTP请求
/// - `is_error_page`: 是否为错误页面（true: 错误页面, false: 404页面）
/// - `RouteResult<Response>`: 成功时返回HTTP响应，失败时返回路由错误
pub async fn custom_page(
    host_route: Ref<'_, String, SettingRoute>,
    request: Request<Body>,
    is_error_page: bool,
) -> RouteResult<Response<Body>> {
    let page = if is_error_page {
        host_route
            .error_page
            .as_ref()
            .ok_or(RouteError::RouteNotFound())?
    } else {
        // 首先尝试查找 not_found_page
        if let Some(page) = host_route.not_found_page.as_ref() {
            page
        } else {
            // 如果 not_found_page 不存在，尝试查找 error_page（用于 404 错误）
            host_route
                .error_page
                .as_ref()
                .ok_or(RouteError::RouteNotFound())?
        }
    };
    debug!("custom_page path {:?}", page);

    let root = host_route
        .root
        .as_ref()
        .ok_or(RouteError::InternalError())?;

    let path = format!("{}/{}", root, page.page);

    let status = StatusCode::from_u16(page.status).map_err(|_| RouteError::BadRequest())?;

    debug!("Custom page path: {:?}", path);

    stream_file(path.into(), request, Some(status))
        .await
        .map_err(|e| {
            error!("Failed to stream custom page: {:?}", e);
            RouteError::InternalError()
        })
}

/// 提供静态文件服务
///
/// 该函数通过以下步骤处理静态文件请求：
/// 1. 从 URI 或提供的路径解析父路径
/// 2. 在 `ROUTE_MAP` 中查找路由以找到根目录
/// 3. 尝试提供请求的文件或默认索引文件
///
/// # 参数
/// - `uri`: 请求的 URI，用于提取完整路径
/// - `path`: 路由器提供的可选路径段
///
/// # 返回值
/// - `Ok(Response)`: 如果文件找到并成功流式传输
/// - `Err(RouteError)`: 如果路由或文件未找到
#[axum::debug_handler]
pub async fn serve(
    uri: Uri,
    path: Option<Path<String>>,
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

    let host = request
        .headers()
        .get("host") // 注意：host 是小写的
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
        
    debug!("Host header: {}", host);

    // parent_path is key in route map
    // which is `host_route.location`
    let parent_path = resolve_parent_path(&uri, path.as_ref());
    let scheme = request.uri().scheme_str().unwrap_or("http");
    // port is key in route_map
    // which is `host_route.port`, used to find current host configuration
    let port = parse_port_from_host(host, scheme).ok_or(RouteError::BadRequest())?;
    // 解析域名
    let (domain, _) = host.split_once(':').unwrap_or((host, ""));
    let domain = domain.to_lowercase();

    let host_config = {
        // 当使用 port: 0 时，会随机分配一个可用端口，但 HOSTS 中存储的键是 0
        let mut port_to_use = port;
        if !HOSTS.contains_key(&port_to_use) {
            port_to_use = 0;
        }
        
        let port_config = HOSTS.get(&port_to_use).ok_or(RouteError::BadRequest())?;

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

        host_config.ok_or(RouteError::BadRequest())?
    };

    let route_map = &host_config.route_map;
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
    // 构建要尝试的候选文件路径列表：
    // - 如果提供了 `path`，则使用它并检查是否是文件
    // - 如果 `path` 为 None，则使用默认索引文件（来自 `host_route.index` 或 `HOST_INDEX`）
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

    // 首先尝试查找索引文件，只有在索引文件不存在时才会考虑自动索引
    let mut index_file_found = false;
    for index_path in &path_arr {
        if fs::metadata(index_path).await.is_ok() {
            index_file_found = true;
            break;
        }
    }
    
    // 如果找到了索引文件，直接提供该文件
    if index_file_found {
        let mut path_exists = None;
        for path in path_arr {
            if fs::metadata(&path).await.is_ok() {
                path_exists = Some(path);
                break;
            }
        }
        return stream_file(path_exists.unwrap().into(), request, None).await;
    }
    
    // 检查是否开启自动生成目录索引，并且索引文件不存在
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
        
        // 检查路径是否存在且可读
        match fs::metadata(&req_path).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    let list = list_dir(&req_path_str, &req_path).await?;
                    // 如果目录是空的，或者 list_dir 返回空列表（表示目录不存在或无法读取），返回 404 错误
                    if list.is_empty() {
                        debug!("Directory {:?} is empty or cannot be read", req_path);
                        return custom_page(host_route, request, false).await;
                    }
                    let list_html = render_list_html(host_root, list);
                    let mut headers = HeaderMap::new();
                    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
                    return Ok((headers, list_html).into_response());
                } else {
                    // 如果路径不是目录，继续处理作为文件请求
                    debug!("Path {:?} is not a directory", req_path);
                }
            },
            Err(_) => {
                // 路径不存在，返回 404 错误
                debug!("Path {:?} does not exist", req_path);
                return custom_page(host_route, request, false).await;
            }
        }
    }

    // 按顺序尝试每个候选路径：
    // - 返回第一个成功流式传输的文件
    // - 如果所有路径都失败，返回 `RouteNotFound` 错误
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
                    .with_context(|| "Insert header failed")?
                    .insert(
                        LOCATION,
                        HeaderValue::from_str(format!("{uri_path}/").as_str())
                            .with_context(|| "Insert header failed")?,
                    );
                response = response.status(StatusCode::MOVED_PERMANENTLY);
                let response = response
                    .body(body)
                    .with_context(|| "Failed to build HTTP response with body")?;
                return Ok(response);
            }
            // 生成自动目录索引
            return if host_route.auto_index {
                // 如果是根路径请求，直接显示目录列表
                // HTML 中的标题路径，需要移除掉配置文件中的 root = "./html" 字段
                let host_root = if let Some(root) = &host_route.root {
                    root
                } else {
                    return custom_page(host_route, request, false).await;
                };
                let req_path_str = req_path.to_string_lossy();
                debug!("auto_index req_path_str: {:?}", req_path_str);
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

/// 生成默认索引文件
/// 如果请求路径不是文件，该函数会读取配置的 index 字段
/// 并与根路径一起构建索引文件数组，如 ["./html/index.html", "./html/index.txt"]
///
/// ## 参数
/// - `host_route`: 主机路由配置
/// - `root`: 根路径
///
/// ## 返回值
/// - PathBuf: 客户端访问的路径
/// - Vec<String>: 包含默认索引文件名的数组
fn generate_default_index(
    host_route: &Ref<'_, String, SettingRoute>,
    root: &str,
) -> (PathBuf, Vec<String>) {
    debug!("host_route.index: {:?}", host_route.index);
    // 如果没有配置索引文件，使用默认的索引文件名
    let indices = if host_route.index.is_empty() {
        vec!["index.html".to_string()].into_iter()
    } else {
        host_route.index.clone().into_iter()
    };
    
    // indices 就是 host_route.index 的中配置的 index 文件名
    let result = (
        root.into(),
        indices.map(|s| format!("{root}/{s}")).collect(),
    );
    debug!("generate_default_index result: {:?}", result);
    result
}

/// 将文件流式传输为 HTTP 响应
///
/// # 参数
/// - `path`: 文件的文件系统路径
///
/// # 返回值
/// - `Ok(Response)`: 如果文件成功打开并流式传输
/// - `Err(anyhow::Error)`: 如果文件无法打开或读取
async fn stream_file(
    path: PathBuf,
    request: Request,
    status: Option<StatusCode>,
) -> RouteResult<Response<Body>> {
    let file = File::open(&path)
        .await
        .with_context(|| "Open file failed")?;

    let path_str = path
        .to_str()
        .ok_or(anyhow!("Convert path to string failed"))?;
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
    let headers = response
        .headers_mut()
        .with_context(|| "Insert header failed")?;
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).with_context(|| "Insert header failed")?,
    );
    headers.insert(
        ETAG,
        HeaderValue::from_str(&etag).with_context(|| "Insert header failed")?,
    );

    let response = if let Some(status) = status {
        response.status(status)
    } else {
        response
    };

    Ok(response
        .body(body)
        .with_context(|| "Failed to build HTTP response with body")?)
}

/// 检查 if-none-match 头部并返回响应
///
/// # 参数
///
/// * `request` - 请求对象
/// * `etag` - 要检查的 ETag
/// * `response` - 响应构建器
///
/// # 返回值
///
/// * `(response, bool)` - 响应构建器和一个布尔值，表示响应是否未修改
pub fn check_if_none_match(request: Request, etag: &String, response: Builder) -> (Builder, bool) {
    // check request if-none-match
    let Some(if_none_match) = request.headers().get(IF_NONE_MATCH) else {
        return (response, false);
    };
    let Ok(if_none_match_str) = if_none_match.to_str() else {
        return (response, false);
    };
    if if_none_match_str == etag {
        return (response.status(StatusCode::NOT_MODIFIED), true);
    }
    (response, false)
}

pub async fn calculate_etag(file: &File, path: &str) -> anyhow::Result<String> {
    let metadata = file
        .metadata()
        .await
        .with_context(|| "Get file metadata failed")?;

    let modified_timestamp = metadata
        .modified()
        .with_context(|| "Get file modified time failed")?
        .duration_since(UNIX_EPOCH)
        .with_context(|| "Calculate Unix timestamp failed")?
        .as_secs();

    // 使用修改时间和文件大小计算ETag（简化且足够唯一）
    let etag_data = format!("{}-{}-{}", path, modified_timestamp, metadata.len());
    let etag = format!("W/\"{:x}\"", md5::compute(etag_data));
    debug!("File {:?} ETag: {:?}", path, etag);

    Ok(etag)
}

// 解析父路径：
// - 如果提供了 `path`，则从 URI 中提取父段
// - 如果 `path` 为 None，则直接使用 URI 路径（确保以 '/' 结尾）
/// 从 URI 和可选的路径段解析父路径
pub fn resolve_parent_path(uri: &Uri, path: Option<&Path<String>>) -> String {
    match path {
        Some(path) => {
            let uri_path = uri.path();
            // 使用路径从URI路径中提取存储在ROUTE_MAP中的父路径
            // uri: /assets/css/styles.07713cb6.css, path: Some(Path("assets/css/styles.07713cb6.css")
            let parent_path = uri_path.get(0..uri_path.len() - path.len());
            parent_path.unwrap_or("/").to_string()
        }
        None => {
            // URI需要以/结尾，因为ROUTE_MAP的键是以/结尾的
            let uri_path = uri.path();
            if uri_path.ends_with('/') {
                uri_path.to_string()
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

    // 检查路径是否存在且是目录
    match fs::metadata(path).await {
        Ok(metadata) => {
            if !metadata.is_dir() {
                debug!("Path {:?} is not a directory", path);
                return Ok(Vec::new());
            }
        },
        Err(_) => {
            debug!("Path {:?} does not exist or is not accessible", path);
            return Ok(Vec::new());
        }
    }

    let mut entries = match fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(e) => {
            debug!("Failed to read directory {:?}: {}", path, e);
            return Ok(Vec::new());
        }
    };

    debug!("列出目录路径: {:?}", path);

    let host_root = host_root_str.to_string();
    let mut tasks = vec![];

    while let Some(entry_result) = entries.next_entry().await? {
        let entry = entry_result;
        let root = host_root.clone();

        let task = tokio::task::spawn(async move {
            let metadata = entry
                .metadata()
                .await
                .with_context(|| "获取文件元数据失败")?;

            let last_modified = metadata
                .modified()
                .with_context(|| "获取文件修改时间失败")?
                .duration_since(UNIX_EPOCH)
                .with_context(|| "计算Unix时间戳失败")?;

            let datetime = match Local
                .timestamp_opt(last_modified.as_secs() as i64, last_modified.subsec_nanos())
            {
                chrono::LocalResult::Ambiguous(earlier, later) => {
                    warn!("检测到歧义时间: {} 和 {}", earlier, later);
                    earlier
                }
                chrono::offset::LocalResult::Single(single) => single,
                chrono::offset::LocalResult::None => {
                    error!("解析时间失败，使用当前时间");
                    Local::now()
                }
            };

            let dir = DirList {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry
                    .path()
                    .to_string_lossy()
                    .strip_prefix(&root)
                    .ok_or(anyhow!("去除路径前缀失败"))?
                    .to_string(),
                is_dir: metadata.is_dir(),
                size: ByteUnit(metadata.len()),
                last_modified: datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            anyhow::Ok(dir)
        });

        tasks.push(task);
    }

    let mut list = vec![];
    for task in tasks {
        match task.await {
            Ok(Ok(mut dir)) => {
                dir.path = if dir.is_dir {
                    format!("./{}/", dir.path)
                } else {
                    format!("./{}", dir.path)
                };
                list.push(dir);
            },
            Ok(Err(e)) => {
                debug!("Failed to process directory entry: {}", e);
            },
            Err(e) => {
                debug!("Task failed to process directory entry: {}", e);
            }
        }
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
        .with_context(|| "Open /dev/null failed")?;
    Ok(ReaderStream::new(empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Path;
    use http::Uri;

    #[test]
    fn test_resolve_parent_path_with_path() {
        // 测试带有路径的情况
        let uri = Uri::try_from("/assets/css/styles.css").unwrap();
        let path = Some(Path("assets/css/styles.css".to_string()));
        let result = resolve_parent_path(&uri, path.as_ref());
        assert_eq!(result, "/");
    }

    #[test]
    fn test_resolve_parent_path_with_subpath() {
        // 测试带有子路径的情况
        let uri = Uri::try_from("/docs/rust/guide.html").unwrap();
        let path = Some(Path("guide.html".to_string()));
        let result = resolve_parent_path(&uri, path.as_ref());
        assert_eq!(result, "/docs/rust/");
    }

    #[test]
    fn test_resolve_parent_path_without_path() {
        // 测试不带路径的情况（不以/结尾）
        let uri = Uri::try_from("/docs").unwrap();
        let result = resolve_parent_path(&uri, None);
        assert_eq!(result, "/docs/");
    }

    #[test]
    fn test_resolve_parent_path_without_path_ends_with_slash() {
        // 测试不带路径且已以/结尾的情况
        let uri = Uri::try_from("/docs/").unwrap();
        let result = resolve_parent_path(&uri, None);
        assert_eq!(result, "/docs/");
    }

    #[test]
    fn test_byte_unit_formatting() {
        // 测试字节单位格式化
        assert_eq!(ByteUnit(0).to_string(), "0 B");
        assert_eq!(ByteUnit(500).to_string(), "500 B");
        assert_eq!(ByteUnit(1024).to_string(), "1024 B");
        assert_eq!(ByteUnit(1500).to_string(), "1.46 KB");
        assert_eq!(ByteUnit(1024 * 1024).to_string(), "1024.00 KB");
        assert_eq!(ByteUnit(1024 * 1024 + 500000).to_string(), "1.48 MB");
        assert_eq!(ByteUnit(1024 * 1024 * 1024).to_string(), "1024.00 MB");
    }

    #[test]
    fn test_check_if_none_match() {
        // 测试 ETag 匹配
        let req = Request::builder()
            .header(IF_NONE_MATCH, "\"12345\"")
            .body(Body::empty())
            .unwrap();
        let etag = "W/\"12345\"".to_string();
        let response = Response::builder();

        let (_res, not_modified) = check_if_none_match(req, &etag, response);
        assert!(!not_modified); // 不匹配，因为前缀不同
    }

    #[test]
    fn test_check_if_none_match_missing() {
        // 测试缺失 If-None-Match 头部
        let req = Request::builder().body(Body::empty()).unwrap();
        let etag = "W/\"12345\"".to_string();
        let response = Response::builder();

        let (_, not_modified) = check_if_none_match(req, &etag, response);
        assert!(!not_modified);
    }
}