use std::sync::{Arc, Mutex};

use http::{HeaderMap, Uri};

/// 为 Lua 脚本提供 HTTP 请求上下文
#[derive(Clone, Debug)]
pub struct CandyRequest {
    /// Uri 在路由中被添加到上下文中
    #[allow(dead_code)]
    pub uri: Uri,
    /// HTTP 版本号 (1.0, 1.1, 2.0, 0.9)
    pub http_version: Option<f32>,
    /// 原始请求头字符串
    pub raw_header: String,
    /// 请求行（如 "GET /t HTTP/1.1"）
    pub request_line: String,
    /// 请求体（原始字节）
    pub body: Arc<Mutex<Option<Vec<u8>>>>,
}

/// 请求的可变状态，使用 Arc<Mutex<>> 共享
#[derive(Clone, Debug)]
pub struct CandyReqState {
    /// 请求方法 (GET, POST, etc.)
    pub method: String,
    /// 当前 URI 路径部分（不含查询参数）
    pub uri_path: String,
    /// 查询参数
    pub uri_args: super::utils::UriArgs,
    /// POST 参数（application/x-www-form-urlencoded）
    pub post_args: Option<super::utils::UriArgs>,
    /// 是否需要重新路由（jump 标志）
    pub jump: bool,
    /// 请求头（可变）
    pub headers: Arc<Mutex<HeaderMap>>,
}

impl CandyReqState {
    /// 构建完整的 URI 字符串
    pub fn build_uri(&self) -> String {
        if self.uri_args.0.is_empty() {
            self.uri_path.clone()
        } else {
            format!("{}?{}", self.uri_path, self.uri_args.to_query())
        }
    }
}

/// 请求操作对象，提供 is_internal 等方法
#[derive(Clone, Debug)]
pub struct CandyReq {
    pub is_internal: bool,
    /// 请求开始时间（秒，包含毫秒小数）
    pub start_time: f64,
    /// HTTP 版本号 (1.0, 1.1, 2.0, 0.9)
    pub http_version: Option<f32>,
    /// 原始请求头字符串
    pub raw_header: String,
    /// 请求行（如 "GET /t HTTP/1.1"）
    pub request_line: String,
    /// 请求体（原始字节）
    pub body: Arc<Mutex<Option<Vec<u8>>>>,
    /// 可变状态（包含请求头）
    pub state: Arc<Mutex<CandyReqState>>,
}

/// HTTP 响应头包装器，支持 Lua 访问
#[derive(Clone, Debug)]
pub struct CandyHeaders {
    pub headers: Arc<Mutex<HeaderMap>>,
}

impl CandyHeaders {
    pub fn new(headers: HeaderMap) -> Self {
        Self {
            headers: Arc::new(Mutex::new(headers)),
        }
    }

    /// 将 Lua 风格的 header 名转换为 HTTP header 名
    /// 下划线转换为连字符，如 content_type -> Content-Type
    pub fn normalize_header_name(key: &str) -> String {
        key.replace('_', "-")
    }

    /// 获取所有 headers 作为 Lua table
    pub fn get_headers_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        let headers = self
            .headers
            .lock()
            .map_err(|e| mlua::Error::external(anyhow::anyhow!("Failed to lock headers: {}", e)))?;

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
pub struct CandyResp {
    pub headers: CandyHeaders,
}

/// 为 Lua 脚本提供 HTTP 响应上下文
#[derive(Clone, Debug)]
pub struct CandyResponse {
    pub status: u16,
    pub headers: CandyHeaders,
    pub body: String,
}

// HTTP 请求上下文，可在 Lua 中使用
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub req: CandyRequest,
    pub res: CandyResponse,
    /// 请求开始时间（秒，包含毫秒小数）
    pub start_time: f64,
    /// 请求的可变状态（方法、URI 等）
    pub req_state: Arc<Mutex<CandyReqState>>,
}
