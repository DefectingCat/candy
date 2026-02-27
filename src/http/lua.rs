use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

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

/// 将自 1970-01-01 以来的天数转换为年月日
fn days_to_ymd(days: i32) -> (i32, u32, u32) {
    // 简化的日期计算算法
    let mut year = 1970;
    let mut remaining_days = days;

    // 计算年份
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // 每月天数
    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    // 计算月份和日期
    let mut month = 1u32;
    let mut day = 1u32;
    for &md in &month_days {
        if remaining_days < md {
            day = remaining_days as u32 + 1;
            break;
        }
        remaining_days -= md;
        month += 1;
    }

    (year, month, day)
}

/// 判断是否为闰年
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// 为 Lua 脚本提供 HTTP 请求上下文
#[derive(Clone, Debug)]
struct CandyRequest {
    /// Uri 在路由中被添加到上下文中
    #[allow(dead_code)]
    uri: Uri,
    /// HTTP 版本号 (1.0, 1.1, 2.0, 0.9)
    http_version: Option<f32>,
    /// 原始请求头字符串
    raw_header: String,
    /// 请求行（如 "GET /t HTTP/1.1"）
    request_line: String,
}

/// 请求的可变状态，使用 Arc<Mutex<>> 共享
#[derive(Clone, Debug)]
struct CandyReqState {
    /// 请求方法 (GET, POST, etc.)
    method: String,
    /// 当前 URI（可被 set_uri 修改）
    uri: String,
    /// 是否需要重新路由（jump 标志）
    jump: bool,
}

/// 请求操作对象，提供 is_internal 等方法
#[derive(Clone, Debug)]
struct CandyReq {
    is_internal: bool,
    /// 请求开始时间（秒，包含毫秒小数）
    start_time: f64,
    /// HTTP 版本号 (1.0, 1.1, 2.0, 0.9)
    http_version: Option<f32>,
    /// 原始请求头字符串
    raw_header: String,
    /// 请求行（如 "GET /t HTTP/1.1"）
    request_line: String,
    /// 可变状态
    state: Arc<Mutex<CandyReqState>>,
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

impl UserData for CandyReq {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // is_internal(): 返回是否为内部请求
        // 在 Candy 中，目前没有子请求机制，始终返回 false
        methods.add_method("is_internal", |_, this, ()| Ok(this.is_internal));

        // start_time(): 返回请求开始时间（秒，包含毫秒小数）
        methods.add_method("start_time", |lua, this, ()| {
            lua.pack(this.start_time)
        });

        // http_version(): 返回 HTTP 版本号
        methods.add_method("http_version", |lua, this, ()| {
            match this.http_version {
                Some(v) => lua.pack(v),
                None => Ok(mlua::Value::Nil),
            }
        });

        // raw_header(no_request_line?): 返回原始请求头
        // raw_header() - 包含请求行
        // raw_header(true) - 不包含请求行
        methods.add_method("raw_header", |lua, this, no_request_line: Option<bool>| {
            let skip_request_line = no_request_line.unwrap_or(false);
            if skip_request_line {
                lua.pack(this.raw_header.clone())
            } else {
                let full = format!("{}\r\n{}", this.request_line, this.raw_header);
                lua.pack(full)
            }
        });

        // get_method(): 返回请求方法名称
        methods.add_method("get_method", |lua, this, ()| {
            let state = this.state.lock().map_err(|e| {
                mlua::Error::external(anyhow!("Failed to lock state: {}", e))
            })?;
            lua.pack(state.method.clone())
        });

        // set_method(method_id): 设置请求方法
        // 使用数字常量，如 cd.HTTP_POST, cd.HTTP_GET
        methods.add_method_mut("set_method", |_, this, method_id: u16| {
            let method = match method_id {
                0 => "GET",
                1 => "HEAD",
                2 => "PUT",
                3 => "POST",
                4 => "DELETE",
                5 => "OPTIONS",
                6 => "MKCOL",
                7 => "COPY",
                8 => "MOVE",
                9 => "PROPFIND",
                10 => "PROPPATCH",
                11 => "LOCK",
                12 => "UNLOCK",
                13 => "PATCH",
                14 => "TRACE",
                _ => {
                    return Err(mlua::Error::external(anyhow!(
                        "Invalid method id: {}",
                        method_id
                    )));
                }
            };
            let mut state = this.state.lock().map_err(|e| {
                mlua::Error::external(anyhow!("Failed to lock state: {}", e))
            })?;
            state.method = method.to_string();
            Ok(())
        });

        // get_uri(): 返回当前 URI
        methods.add_method("get_uri", |lua, this, ()| {
            let state = this.state.lock().map_err(|e| {
                mlua::Error::external(anyhow!("Failed to lock state: {}", e))
            })?;
            lua.pack(state.uri.clone())
        });

        // set_uri(uri, jump?): 设置当前 URI
        // jump=true 时标记需要重新路由（类似 nginx rewrite ... last）
        methods.add_method_mut("set_uri", |_, this, (uri, jump): (String, Option<bool>)| {
            if uri.is_empty() {
                return Err(mlua::Error::external(anyhow!(
                    "uri argument must be a non-empty string"
                )));
            }
            let mut state = this.state.lock().map_err(|e| {
                mlua::Error::external(anyhow!("Failed to lock state: {}", e))
            })?;
            state.uri = uri;
            state.jump = jump.unwrap_or(false);
            Ok(())
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
    /// 请求开始时间（秒，包含毫秒小数）
    start_time: f64,
    /// 请求的可变状态（方法、URI 等）
    req_state: Arc<Mutex<CandyReqState>>,
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
                "req" => {
                    // 返回 req 对象，提供 is_internal 等方法
                    lua.create_userdata(CandyReq {
                        is_internal: false,
                        start_time: this.start_time,
                        http_version: this.req.http_version,
                        raw_header: this.req.raw_header.clone(),
                        request_line: this.req.request_line.clone(),
                        state: this.req_state.clone(),
                    })
                    .map(mlua::Value::UserData)
                }
                "now" => {
                    // now(): 返回当前时间戳（秒，包含毫秒小数部分）
                    let now_func = lua.create_function(|lua, ()| {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map_err(|e| mlua::Error::external(anyhow!("Time error: {}", e)))?;
                        let secs = now.as_secs() as f64 + now.subsec_nanos() as f64 / 1_000_000_000.0;
                        lua.pack(secs)
                    })?;
                    Ok(mlua::Value::Function(now_func))
                }
                "time" => {
                    // time(): 返回当前时间戳（整数秒）
                    let time_func = lua.create_function(|lua, ()| {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map_err(|e| mlua::Error::external(anyhow!("Time error: {}", e)))?;
                        lua.pack(now.as_secs())
                    })?;
                    Ok(mlua::Value::Function(time_func))
                }
                "today" => {
                    // today(): 返回当前日期（格式 yyyy-mm-dd）
                    let today_func = lua.create_function(|lua, ()| {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map_err(|e| mlua::Error::external(anyhow!("Time error: {}", e)))?;
                        let secs = now.as_secs();
                        // 计算日期（简化实现，不处理时区）
                        let days = secs / 86400;
                        // 从 1970-01-01 开始计算
                        let (year, month, day) = days_to_ymd(days as i32);
                        let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
                        lua.pack(date_str)
                    })?;
                    Ok(mlua::Value::Function(today_func))
                }
                "update_time" => {
                    // update_time(): 强制更新时间（在 Candy 中是空操作，因为每次都获取最新时间）
                    let update_time_func = lua.create_function(|_, ()| {
                        // Candy 每次调用 now()/today() 都会获取最新时间
                        // 此函数仅为 API 兼容性而存在
                        Ok(())
                    })?;
                    Ok(mlua::Value::Function(update_time_func))
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

    // 解析 HTTP 版本号
    let http_version = match req.version() {
        http::Version::HTTP_09 => Some(0.9),
        http::Version::HTTP_10 => Some(1.0),
        http::Version::HTTP_11 => Some(1.1),
        http::Version::HTTP_2 => Some(2.0),
        http::Version::HTTP_3 => Some(3.0),
        _ => None,
    };

    // 构建 HTTP 版本字符串
    let http_version_str = match req.version() {
        http::Version::HTTP_09 => "HTTP/0.9",
        http::Version::HTTP_10 => "HTTP/1.0",
        http::Version::HTTP_11 => "HTTP/1.1",
        http::Version::HTTP_2 => "HTTP/2.0",
        http::Version::HTTP_3 => "HTTP/3.0",
        _ => "HTTP/1.1",
    };

    // 构建请求行
    let request_line = format!(
        "{} {} {}",
        method,
        req_uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/"),
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

    let lua = &LUA_ENGINE.lua;
    let script = fs::read_to_string(lua_script)
        .await
        .with_context(|| format!("Failed to read lua script file: {lua_script}",))?;

    // 计算请求开始时间
    let start_time = {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RouteError::InternalError())?;
        now.as_secs() as f64 + now.subsec_nanos() as f64 / 1_000_000_000.0
    };

    // 创建请求的可变状态
    let req_state = Arc::new(Mutex::new(CandyReqState {
        method: method.clone(),
        uri: req_uri.path_and_query().map(|pq| pq.to_string()).unwrap_or_default(),
        jump: false,
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
                },
                res: CandyResponse {
                    status: 200,
                    headers: CandyHeaders::new(HeaderMap::new()),
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
