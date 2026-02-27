use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use http::Uri;
use tokio::fs;
use tracing::error;

use crate::{
    http::{HOSTS, error::RouteError, serve::resolve_parent_path},
    lua_engine::LUA_ENGINE,
    utils::parse_port_from_host,
};

use super::{
    structures::{CandyReqState, CandyRequest, CandyResponse, RequestContext},
    utils::UriArgs,
};
use crate::http::error::RouteResult;

pub async fn lua(
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

    // 解析域名（提前计算，避免借用冲突）
    let (domain, _) = host.split_once(':').unwrap_or((host, ""));
    let domain = domain.to_lowercase();

    let port = parse_port_from_host(host, scheme).ok_or(RouteError::BadRequest())?;

    // 提取请求方法（在消费 req 之前）
    let method = req.method().to_string();

    // 解析 HTTP 版本号
    let http_version = match req.version() {
        http::Version::HTTP_09 => Some(0.9),
        http::Version::HTTP_10 => Some(1.0),
        http::Version::HTTP_11 => Some(1.1),
        http::Version::HTTP_2 => Some(2.0),
        http::Version::HTTP_3 => Some(3.0),
        _ => None,
    };

    // 构建请求行所需信息
    let http_version_str = match req.version() {
        http::Version::HTTP_09 => "HTTP/0.9",
        http::Version::HTTP_10 => "HTTP/1.0",
        http::Version::HTTP_11 => "HTTP/1.1",
        http::Version::HTTP_2 => "HTTP/2.0",
        http::Version::HTTP_3 => "HTTP/3.0",
        _ => "HTTP/1.1",
    };
    let request_line = format!(
        "{} {} {}",
        method,
        req_uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/"),
        http_version_str
    );

    // 构建原始请求头字符串
    let raw_header = {
        let mut headers_str = String::new();
        for (name, value) in req.headers() {
            if let Ok(v) = value.to_str() {
                headers_str.push_str(&format!("{}: {}\r\n", name, v));
            }
        }
        headers_str
    };

    // 克隆请求头（用于 get_headers）
    let req_headers = req.headers().clone();

    // 收集请求体（用于 POST 参数解析）
    let body_bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024) // 限制 10MB
        .await
        .map_err(|e| RouteError::Any(anyhow!("Failed to read body: {}", e)))?;
    let req_body = Arc::new(Mutex::new(Some(body_bytes.to_vec())));

    let host_config = {
        let port_config = HOSTS.get(&port).ok_or(RouteError::BadRequest())?;

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
    tracing::debug!("Lua: Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    let route_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())?;
    let lua_script = route_config
        .lua_script
        .as_ref()
        .ok_or(RouteError::InternalError())?;

    // 计算请求开始时间
    let start_time = {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RouteError::InternalError())?;
        now.as_secs() as f64 + now.subsec_nanos() as f64 / 1_000_000_000.0
    };

    let lua = &LUA_ENGINE.lua;
    let script = fs::read_to_string(lua_script)
        .await
        .with_context(|| format!("Failed to read lua script file: {lua_script}",))?;

    // 创建请求的可变状态
    let (uri_path, uri_args) = req_uri
        .path_and_query()
        .map(|pq| {
            let (path, query) = pq.as_str().split_once('?').unwrap_or((pq.as_str(), ""));
            (path.to_string(), UriArgs::from_query(query))
        })
        .unwrap_or_else(|| ("/".to_string(), UriArgs::new()));

    let req_state = Arc::new(Mutex::new(CandyReqState {
        method,
        uri_path,
        uri_args,
        post_args: None,
        jump: false,
        headers: Arc::new(Mutex::new(req_headers)),
    }));

    lua.globals()
        .set(
            "cd",
            RequestContext {
                req: CandyRequest {
                    uri: req_uri,
                    http_version,
                    raw_header,
                    request_line,
                    body: req_body,
                },
                res: CandyResponse {
                    status: 200,
                    headers: super::structures::CandyHeaders::new(http::HeaderMap::new()),
                    body: "".to_string(),
                },
                start_time,
                req_state,
            },
        )
        .map_err(|err| {
            error!("Lua script {lua_script} exec error: {err}");
            RouteError::InternalError()
        })?;
    lua.load(script).exec_async().await.map_err(|err| {
        error!("Lua script {lua_script} exec error: {err}");
        RouteError::InternalError()
    })?;
    // 获取修改后的上下文并返回响应
    let ctx: mlua::UserDataRef<RequestContext> = lua.globals().get("cd").map_err(|err| {
        error!("Lua script {lua_script} exec error: {err}");
        RouteError::InternalError()
    })?;
    let res = ctx.res.clone();

    let mut response = Response::builder();
    let body = Body::from(res.body);
    response = response.status(res.status);

    // 添加响应头
    let headers = response.headers_mut().unwrap();
    if let Ok(guard) = res.headers.headers.lock() {
        for (name, value) in guard.iter() {
            headers.append(name.clone(), value.clone());
        }
    }

    let response = response
        .body(body)
        .with_context(|| "Failed to build HTTP response with lua")?;
    Ok(response)
}
