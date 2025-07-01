use std::str::FromStr;

use anyhow::{Context, anyhow};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use http::{HeaderMap, HeaderName, HeaderValue, Uri};
use mlua::{UserData, UserDataMethods, UserDataRef};
use tokio::fs::{self};
use tracing::error;

use crate::{
    http::{HOSTS, LUA_ENGINE, error::RouteError, serve::resolve_parent_path},
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
        // 获取请求路径
        methods.add_method("get_path", |_, this, ()| {
            Ok(this.req.uri.path().to_string())
        });

        // 获取请求方法
        methods.add_method("get_method", |_, this, ()| Ok(this.req.method.to_string()));

        // 设置响应状态码
        methods.add_method_mut("set_status", |_, this, status: u16| {
            this.res.status = status;
            Ok(())
        });

        // 设置响应内容
        methods.add_method_mut("set_body", |_, this, body: String| {
            this.res.body = format!("{}{}", this.res.body, body);
            Ok(())
        });

        // 设置响应头
        methods.add_method_mut("set_header", |_, this, (key, value): (String, String)| {
            this.res.headers.insert(
                HeaderName::from_str(&key).map_err(|err| anyhow!("header name error: {err}"))?,
                HeaderValue::from_str(&value)
                    .map_err(|err| anyhow!("header value error: {err}"))?,
            );
            Ok(())
        });
    }
}

pub async fn lua(
    req_uri: Uri,
    path: Option<Path<String>>,
    Host(host): Host,
    req: Request<Body>,
) -> RouteResult<impl IntoResponse> {
    let scheme = req.uri().scheme_str().unwrap_or("http");
    let port = parse_port_from_host(&host, scheme).ok_or(RouteError::BadRequest())?;
    let route_map = &HOSTS
        .get(&port)
        .ok_or(RouteError::BadRequest())
        .with_context(|| {
            format!("Hosts not found for port: {port}, host: {host}, scheme: {scheme}")
        })?
        .route_map;
    tracing::debug!("Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    let route_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())
        .with_context(|| format!("route not found: {parent_path}"))?;
    let lua_script = route_config
        .lua_script
        .as_ref()
        .ok_or(RouteError::InternalError())
        .with_context(|| "lua script not found")?;

    let method = req.method().to_string();

    let lua = &LUA_ENGINE.lua;
    let script = fs::read_to_string(lua_script)
        .await
        .with_context(|| format!("Failed to read lua script file: {lua_script}",))?;
    lua.globals()
        .set(
            "ctx",
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
    let ctx: UserDataRef<RequestContext> = lua.globals().get("ctx").map_err(|err| {
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
