use std::sync::{Arc, Mutex};

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

/// HTTP 响应头包装器，支持 Lua 访问
#[derive(Clone, Debug)]
struct CandyHeaders {
    headers: Arc<Mutex<HeaderMap>>,
}

impl CandyHeaders {
    fn new(headers: HeaderMap) -> Self {
        Self {
            headers: Arc::new(Mutex::new(headers)),
        }
    }

    /// 将 Lua 风格的 header 名转换为 HTTP header 名
    /// 下划线转换为连字符，如 content_type -> Content-Type
    fn normalize_header_name(key: &str) -> String {
        key.replace('_', "-")
    }

    /// 获取所有 headers 作为 Lua table
    fn get_headers_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        let headers = self.headers.lock().map_err(|e| {
            mlua::Error::external(anyhow!("Failed to lock headers: {}", e))
        })?;

        let table = lua.create_table()?;
        for (name, value) in headers.iter() {
            let key = name.as_str();
            if let Ok(v) = value.to_str() {
                // 如果已有相同 key，转换为数组
                if table.contains_key(key)? {
                    let existing: mlua::Value = table.get(key)?;
                    match existing {
                        mlua::Value::String(s) => {
                            let arr = lua.create_table()?;
                            arr.set(1, s)?;
                            arr.set(2, v)?;
                            table.set(key, arr)?;
                        }
                        mlua::Value::Table(t) => {
                            let len = t.len()?;
                            t.set(len + 1, v)?;
                        }
                        _ => {}
                    }
                } else {
                    table.set(key, v)?;
                }
            }
        }
        Ok(table)
    }
}

/// 响应操作对象，提供 get_headers 等方法
#[derive(Clone, Debug)]
struct CandyResp {
    headers: CandyHeaders,
}

impl UserData for CandyResp {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // get_headers(): 返回所有响应头的 table
        methods.add_method("get_headers", |lua, this, ()| {
            this.headers.get_headers_table(lua)
        });
    }
}

impl UserData for CandyHeaders {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // __index: 读取 header
        // 支持 cd.header["Content-Type"] 和 cd.header.content_type
        methods.add_meta_method("__index", |lua, this, key: String| {
            let normalized = Self::normalize_header_name(&key);
            let headers = this.headers.lock().map_err(|e| {
                mlua::Error::external(anyhow!("Failed to lock headers: {}", e))
            })?;

            // 查找 header (大小写不敏感)
            let header_name = HeaderName::try_from(normalized.as_str())
                .map_err(|e| mlua::Error::external(anyhow!("Invalid header name: {}", e)))?;

            let values: Vec<String> = headers
                .get_all(&header_name)
                .iter()
                .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
                .collect();

            if values.is_empty() {
                Ok(mlua::Value::Nil)
            } else if values.len() == 1 {
                Ok(mlua::Value::String(lua.create_string(&values[0])?))
            } else {
                // 多值 header 返回 table
                let table = lua.create_table()?;
                for (i, v) in values.iter().enumerate() {
                    table.set(i + 1, v.clone())?;
                }
                Ok(mlua::Value::Table(table))
            }
        });

        // __newindex: 设置/删除 header
        // cd.header["Content-Type"] = "text/plain"
        // cd.header["Set-Cookie"] = {"a=1", "b=2"}
        // cd.header["X-My-Header"] = nil  -- 删除
        methods.add_meta_method_mut("__newindex", |_lua, this, (key, value): (String, mlua::Value)| {
            let normalized = Self::normalize_header_name(&key);
            let header_name = HeaderName::try_from(normalized.as_str())
                .map_err(|e| mlua::Error::external(anyhow!("Invalid header name: {}", e)))?;

            let mut headers = this.headers.lock().map_err(|e| {
                mlua::Error::external(anyhow!("Failed to lock headers: {}", e))
            })?;

            // 先移除已有的值
            headers.remove(&header_name);

            match value {
                mlua::Value::Nil => {
                    // 删除 header，已经 remove 了，不需要额外操作
                }
                mlua::Value::String(s) => {
                    let val = s.to_str()?;
                    let header_value = HeaderValue::from_str(&val)
                        .map_err(|e| mlua::Error::external(anyhow!("Invalid header value: {}", e)))?;
                    headers.append(header_name.clone(), header_value);
                }
                mlua::Value::Table(t) => {
                    // 多值 header
                    for pair in t.pairs::<i32, mlua::String>() {
                        let (_, v) = pair.map_err(|e| {
                            mlua::Error::external(anyhow!("Invalid header value in table: {}", e))
                        })?;
                        let val = v.to_str()?;
                        let header_value = HeaderValue::from_str(&val)
                            .map_err(|e| mlua::Error::external(anyhow!("Invalid header value: {}", e)))?;
                        headers.append(header_name.clone(), header_value);
                    }
                }
                _ => {
                    return Err(mlua::Error::external(anyhow!(
                        "Header value must be string, table, or nil"
                    )));
                }
            }

            Ok(())
        });
    }
}

/// 为 Lua 脚本提供 HTTP 响应上下文
#[derive(Clone, Debug)]
struct CandyResponse {
    status: u16,
    headers: CandyHeaders,
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
        // 元方法：实现属性访问 (cd.status, cd.header)
        // 注意：需要同时处理常量字段和动态属性
        methods.add_meta_method("__index", |lua, this, key: String| {
            match key.as_str() {
                // 动态属性
                "status" => lua.pack(this.res.status),
                "header" => {
                    // 返回 headers 对象
                    lua.create_userdata(this.res.headers.clone())
                        .map(mlua::Value::UserData)
                }
                "resp" => {
                    // 返回 resp 对象，提供 get_headers 方法
                    lua.create_userdata(CandyResp {
                        headers: this.res.headers.clone(),
                    })
                    .map(mlua::Value::UserData)
                }
                // HTTP 方法常量
                "HTTP_GET" => lua.pack(0u16),
                "HTTP_HEAD" => lua.pack(1u16),
                "HTTP_PUT" => lua.pack(2u16),
                "HTTP_POST" => lua.pack(3u16),
                "HTTP_DELETE" => lua.pack(4u16),
                "HTTP_OPTIONS" => lua.pack(5u16),
                "HTTP_MKCOL" => lua.pack(6u16),
                "HTTP_COPY" => lua.pack(7u16),
                "HTTP_MOVE" => lua.pack(8u16),
                "HTTP_PROPFIND" => lua.pack(9u16),
                "HTTP_PROPPATCH" => lua.pack(10u16),
                "HTTP_LOCK" => lua.pack(11u16),
                "HTTP_UNLOCK" => lua.pack(12u16),
                "HTTP_PATCH" => lua.pack(13u16),
                "HTTP_TRACE" => lua.pack(14u16),
                // HTTP 状态码常量 - 1xx
                "HTTP_CONTINUE" => lua.pack(100u16),
                "HTTP_SWITCHING_PROTOCOLS" => lua.pack(101u16),
                // HTTP 状态码常量 - 2xx
                "HTTP_OK" => lua.pack(200u16),
                "HTTP_CREATED" => lua.pack(201u16),
                "HTTP_ACCEPTED" => lua.pack(202u16),
                "HTTP_NO_CONTENT" => lua.pack(204u16),
                "HTTP_PARTIAL_CONTENT" => lua.pack(206u16),
                // HTTP 状态码常量 - 3xx
                "HTTP_SPECIAL_RESPONSE" => lua.pack(300u16),
                "HTTP_MOVED_PERMANENTLY" => lua.pack(301u16),
                "HTTP_MOVED_TEMPORARILY" => lua.pack(302u16),
                "HTTP_SEE_OTHER" => lua.pack(303u16),
                "HTTP_NOT_MODIFIED" => lua.pack(304u16),
                "HTTP_TEMPORARY_REDIRECT" => lua.pack(307u16),
                // HTTP 状态码常量 - 4xx
                "HTTP_BAD_REQUEST" => lua.pack(400u16),
                "HTTP_UNAUTHORIZED" => lua.pack(401u16),
                "HTTP_PAYMENT_REQUIRED" => lua.pack(402u16),
                "HTTP_FORBIDDEN" => lua.pack(403u16),
                "HTTP_NOT_FOUND" => lua.pack(404u16),
                "HTTP_NOT_ALLOWED" => lua.pack(405u16),
                "HTTP_NOT_ACCEPTABLE" => lua.pack(406u16),
                "HTTP_REQUEST_TIMEOUT" => lua.pack(408u16),
                "HTTP_CONFLICT" => lua.pack(409u16),
                "HTTP_GONE" => lua.pack(410u16),
                "HTTP_UPGRADE_REQUIRED" => lua.pack(426u16),
                "HTTP_TOO_MANY_REQUESTS" => lua.pack(429u16),
                "HTTP_CLOSE" => lua.pack(444u16),
                "HTTP_ILLEGAL" => lua.pack(451u16),
                // HTTP 状态码常量 - 5xx
                "HTTP_INTERNAL_SERVER_ERROR" => lua.pack(500u16),
                "HTTP_METHOD_NOT_IMPLEMENTED" => lua.pack(501u16),
                "HTTP_BAD_GATEWAY" => lua.pack(502u16),
                "HTTP_SERVICE_UNAVAILABLE" => lua.pack(503u16),
                "HTTP_GATEWAY_TIMEOUT" => lua.pack(504u16),
                "HTTP_VERSION_NOT_SUPPORTED" => lua.pack(505u16),
                "HTTP_INSUFFICIENT_STORAGE" => lua.pack(507u16),
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
                    headers: CandyHeaders::new(HeaderMap::new()),
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
