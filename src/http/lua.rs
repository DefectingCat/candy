use std::str::FromStr;

use anyhow::{Context, anyhow};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use http::{HeaderMap, HeaderName, HeaderValue, Uri};
use mlua::{UserData, UserDataMethods, UserDataRef};
use tokio::fs::{self};
use tracing::error;

use crate::{
    http::{HOSTS, error::RouteError, serve::resolve_parent_path},
    lua_engine::LUA_ENGINE,
    utils::parse_port_from_host,
};

use super::error::RouteResult;

/// 为 Lua 脚本提供 HTTP 请求上下文
#[derive(Clone, Debug)]
struct CandyRequest {
    #[allow(dead_code)]
    method: String,
    /// Uri 在路由中被添加到上下文中
    uri: Uri,
}
/// 为 Lua 脚本提供 HTTP 响应上下文
#[derive(Clone, Debug)]
struct CandyResponse {
    status: u16,
    headers: HeaderMap,
    body: String,
}
// HTTP 请求上下文，可在 Lua 中使用
#[derive(Clone, Debug)]
struct RequestContext {
    req: CandyRequest,
    res: CandyResponse,
}

impl UserData for RequestContext {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // 元方法：实现属性访问 (cd.status)
        // 注意：需要同时处理常量字段和动态属性
        methods.add_meta_method("__index", |_, this, key: String| {
            match key.as_str() {
                // 动态属性
                "status" => Ok(this.res.status),
                // HTTP 方法常量
                "HTTP_GET" => Ok(0u16),
                "HTTP_HEAD" => Ok(1u16),
                "HTTP_PUT" => Ok(2u16),
                "HTTP_POST" => Ok(3u16),
                "HTTP_DELETE" => Ok(4u16),
                "HTTP_OPTIONS" => Ok(5u16),
                "HTTP_MKCOL" => Ok(6u16),
                "HTTP_COPY" => Ok(7u16),
                "HTTP_MOVE" => Ok(8u16),
                "HTTP_PROPFIND" => Ok(9u16),
                "HTTP_PROPPATCH" => Ok(10u16),
                "HTTP_LOCK" => Ok(11u16),
                "HTTP_UNLOCK" => Ok(12u16),
                "HTTP_PATCH" => Ok(13u16),
                "HTTP_TRACE" => Ok(14u16),
                // HTTP 状态码常量 - 1xx
                "HTTP_CONTINUE" => Ok(100u16),
                "HTTP_SWITCHING_PROTOCOLS" => Ok(101u16),
                // HTTP 状态码常量 - 2xx
                "HTTP_OK" => Ok(200u16),
                "HTTP_CREATED" => Ok(201u16),
                "HTTP_ACCEPTED" => Ok(202u16),
                "HTTP_NO_CONTENT" => Ok(204u16),
                "HTTP_PARTIAL_CONTENT" => Ok(206u16),
                // HTTP 状态码常量 - 3xx
                "HTTP_SPECIAL_RESPONSE" => Ok(300u16),
                "HTTP_MOVED_PERMANENTLY" => Ok(301u16),
                "HTTP_MOVED_TEMPORARILY" => Ok(302u16),
                "HTTP_SEE_OTHER" => Ok(303u16),
                "HTTP_NOT_MODIFIED" => Ok(304u16),
                "HTTP_TEMPORARY_REDIRECT" => Ok(307u16),
                // HTTP 状态码常量 - 4xx
                "HTTP_BAD_REQUEST" => Ok(400u16),
                "HTTP_UNAUTHORIZED" => Ok(401u16),
                "HTTP_PAYMENT_REQUIRED" => Ok(402u16),
                "HTTP_FORBIDDEN" => Ok(403u16),
                "HTTP_NOT_FOUND" => Ok(404u16),
                "HTTP_NOT_ALLOWED" => Ok(405u16),
                "HTTP_NOT_ACCEPTABLE" => Ok(406u16),
                "HTTP_REQUEST_TIMEOUT" => Ok(408u16),
                "HTTP_CONFLICT" => Ok(409u16),
                "HTTP_GONE" => Ok(410u16),
                "HTTP_UPGRADE_REQUIRED" => Ok(426u16),
                "HTTP_TOO_MANY_REQUESTS" => Ok(429u16),
                "HTTP_CLOSE" => Ok(444u16),
                "HTTP_ILLEGAL" => Ok(451u16),
                // HTTP 状态码常量 - 5xx
                "HTTP_INTERNAL_SERVER_ERROR" => Ok(500u16),
                "HTTP_METHOD_NOT_IMPLEMENTED" => Ok(501u16),
                "HTTP_BAD_GATEWAY" => Ok(502u16),
                "HTTP_SERVICE_UNAVAILABLE" => Ok(503u16),
                "HTTP_GATEWAY_TIMEOUT" => Ok(504u16),
                "HTTP_VERSION_NOT_SUPPORTED" => Ok(505u16),
                "HTTP_INSUFFICIENT_STORAGE" => Ok(507u16),
                _ => Err(mlua::Error::external(anyhow!(
                    "attempt to index unknown field: {}",
                    key
                ))),
            }
        });

        // 元方法：实现属性设置 (cd.status = 200)
        methods.add_meta_method_mut("__newindex", |_, this, (key, value): (String, u16)| {
            match key.as_str() {
                "status" => {
                    this.res.status = value;
                    Ok(())
                }
                _ => Err(mlua::Error::external(anyhow!(
                    "attempt to set unknown field: {}",
                    key
                ))),
            }
        });
    }
}

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
    let port = parse_port_from_host(host, scheme).ok_or(RouteError::BadRequest())?;
    // 解析域名
    let (domain, _) = host.split_once(':').unwrap_or((host, ""));
    let domain = domain.to_lowercase();

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

    let method = req.method().to_string();

    let lua = &LUA_ENGINE.lua;
    let script = fs::read_to_string(lua_script)
        .await
        .with_context(|| format!("Failed to read lua script file: {lua_script}",))?;
    lua.globals()
        .set(
            "cd",
            RequestContext {
                req: CandyRequest {
                    method,
                    uri: req_uri,
                },
                res: CandyResponse {
                    status: 200,
                    headers: HeaderMap::new(),
                    body: "".to_string(),
                },
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
    let ctx: UserDataRef<RequestContext> = lua.globals().get("cd").map_err(|err| {
        error!("Lua script {lua_script} exec error: {err}");
        RouteError::InternalError()
    })?;
    let res = ctx.res.clone();

    let mut response = Response::builder();
    let body = Body::from(res.body);
    response = response.status(res.status);
    let response = response
        .body(body)
        .with_context(|| "Failed to build HTTP response with lua")?;
    Ok(response)
}
