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
    /// 重定向 URL（如果设置则需要重定向）
    pub redirect_uri: Option<String>,
    /// 重定向状态码
    pub redirect_status: Option<u16>,
    /// 通过 print/say 输出的内容
    pub output_buffer: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    // CandyReqState tests
    mod candy_req_state {
        use super::*;
        use crate::http::lua::utils::UriArgs;

        #[test]
        fn test_build_uri_without_args() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/test".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };
            assert_eq!(state.build_uri(), "/test");
        }

        #[test]
        fn test_build_uri_with_args() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/test".to_string(),
                uri_args: UriArgs(vec![
                    ("key1".to_string(), "value1".to_string()),
                    ("key2".to_string(), "value2".to_string()),
                ]),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };
            assert_eq!(state.build_uri(), "/test?key1=value1&key2=value2");
        }

        #[test]
        fn test_build_uri_with_empty_value() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/api".to_string(),
                uri_args: UriArgs(vec![("flag".to_string(), "".to_string())]),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };
            assert_eq!(state.build_uri(), "/api?flag");
        }

        #[test]
        fn test_build_uri_with_special_chars() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/search".to_string(),
                uri_args: UriArgs(vec![(
                    "q".to_string(),
                    "hello world".to_string(),
                )]),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };
            let uri = state.build_uri();
            assert!(uri.contains("q="));
        }
    }

    // CandyHeaders tests
    mod candy_headers {
        use super::*;

        #[test]
        fn test_new() {
            let headers = HeaderMap::new();
            let candy_headers = CandyHeaders::new(headers);
            assert!(candy_headers.headers.lock().is_ok());
        }

        #[test]
        fn test_normalize_header_name_no_change() {
            assert_eq!(CandyHeaders::normalize_header_name("content-type"), "content-type");
            assert_eq!(CandyHeaders::normalize_header_name("accept"), "accept");
            assert_eq!(CandyHeaders::normalize_header_name("host"), "host");
        }

        #[test]
        fn test_normalize_header_name_underscore_to_dash() {
            assert_eq!(CandyHeaders::normalize_header_name("content_type"), "content-type");
            assert_eq!(CandyHeaders::normalize_header_name("accept_encoding"), "accept-encoding");
            assert_eq!(CandyHeaders::normalize_header_name("x_custom_header"), "x-custom-header");
        }

        #[test]
        fn test_normalize_header_name_mixed() {
            assert_eq!(CandyHeaders::normalize_header_name("Content_Type"), "Content-Type");
            assert_eq!(CandyHeaders::normalize_header_name("X_API_KEY"), "X-API-KEY");
        }

        #[test]
        fn test_normalize_header_name_empty() {
            assert_eq!(CandyHeaders::normalize_header_name(""), "");
        }

        #[test]
        fn test_normalize_header_name_multiple_underscores() {
            assert_eq!(
                CandyHeaders::normalize_header_name("x_a_b_c"),
                "x-a-b-c"
            );
        }

        #[test]
        fn test_headers_access() {
            let mut headers = HeaderMap::new();
            headers.insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            headers.insert(
                http::header::CONTENT_LENGTH,
                HeaderValue::from_static("100"),
            );

            let candy_headers = CandyHeaders::new(headers);
            let guard = candy_headers.headers.lock().unwrap();

            assert_eq!(guard.get(http::header::CONTENT_TYPE).unwrap(), "application/json");
            assert_eq!(guard.get(http::header::CONTENT_LENGTH).unwrap(), "100");
        }
    }

    // CandyRequest tests
    mod candy_request {
        use super::*;

        #[test]
        fn test_default_values() {
            let uri = Uri::from_static("/test");
            let request = CandyRequest {
                uri,
                http_version: Some(1.1),
                raw_header: "Host: localhost\r\n".to_string(),
                request_line: "GET /test HTTP/1.1".to_string(),
                body: Arc::new(Mutex::new(Some(b"test".to_vec()))),
            };

            assert_eq!(request.uri.path(), "/test");
            assert_eq!(request.http_version, Some(1.1));
            assert!(!request.raw_header.is_empty());
            assert!(request.request_line.contains("GET"));
        }

        #[test]
        fn test_body_access() {
            let body_data = b"Hello World";
            let request = CandyRequest {
                uri: Uri::from_static("/"),
                http_version: Some(1.1),
                raw_header: String::new(),
                request_line: "GET / HTTP/1.1".to_string(),
                body: Arc::new(Mutex::new(Some(body_data.to_vec()))),
            };

            let guard = request.body.lock().unwrap();
            assert_eq!(guard.as_ref().unwrap(), body_data);
        }

        #[test]
        fn test_body_empty() {
            let request = CandyRequest {
                uri: Uri::from_static("/"),
                http_version: Some(1.1),
                raw_header: String::new(),
                request_line: "GET / HTTP/1.1".to_string(),
                body: Arc::new(Mutex::new(None)),
            };

            let guard = request.body.lock().unwrap();
            assert!(guard.is_none());
        }
    }

    // CandyResponse tests
    mod candy_response {
        use super::*;

        #[test]
        fn test_default_values() {
            let response = CandyResponse {
                status: 200,
                headers: CandyHeaders::new(HeaderMap::new()),
                body: "Hello".to_string(),
            };

            assert_eq!(response.status, 200);
            assert_eq!(response.body, "Hello");
        }

        #[test]
        fn test_with_headers() {
            let mut headers = HeaderMap::new();
            headers.insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/html"),
            );

            let response = CandyResponse {
                status: 404,
                headers: CandyHeaders::new(headers),
                body: "Not Found".to_string(),
            };

            assert_eq!(response.status, 404);
            assert_eq!(response.body, "Not Found");
        }
    }

    // RequestContext tests
    mod request_context {
        use super::*;
        use crate::http::lua::utils::UriArgs;

        #[test]
        fn test_full_context_creation() {
            let uri = Uri::from_static("/test");
            let req_body = Arc::new(Mutex::new(Some(b"body data".to_vec())));

            let req_state = Arc::new(Mutex::new(CandyReqState {
                method: "POST".to_string(),
                uri_path: "/test".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            }));

            let ctx = RequestContext {
                req: CandyRequest {
                    uri,
                    http_version: Some(1.1),
                    raw_header: "".to_string(),
                    request_line: "POST /test HTTP/1.1".to_string(),
                    body: req_body.clone(),
                },
                res: CandyResponse {
                    status: 200,
                    headers: CandyHeaders::new(HeaderMap::new()),
                    body: "response".to_string(),
                },
                start_time: 1234567890.0,
                req_state,
            };

            assert_eq!(ctx.start_time, 1234567890.0);
            assert_eq!(ctx.res.status, 200);
            assert_eq!(ctx.res.body, "response");
        }

        #[test]
        fn test_context_clone() {
            let req_body = Arc::new(Mutex::new(Some(b"test".to_vec())));
            let headers = Arc::new(Mutex::new(HeaderMap::new()));

            let req_state = Arc::new(Mutex::new(CandyReqState {
                method: "GET".to_string(),
                uri_path: "/".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers: headers.clone(),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            }));

            let ctx = RequestContext {
                req: CandyRequest {
                    uri: Uri::from_static("/"),
                    http_version: Some(1.1),
                    raw_header: String::new(),
                    request_line: "GET / HTTP/1.1".to_string(),
                    body: req_body.clone(),
                },
                res: CandyResponse {
                    status: 200,
                    headers: CandyHeaders::new(HeaderMap::new()),
                    body: "OK".to_string(),
                },
                start_time: 1000.0,
                req_state: req_state.clone(),
            };

            // Clone and verify
            let ctx_clone = ctx.clone();
            assert_eq!(ctx_clone.start_time, 1000.0);
            assert_eq!(ctx_clone.res.body, "OK");
        }
    }

    // Integration tests - CandyReqState with headers
    mod candy_req_with_state {
        use super::*;
        use crate::http::lua::utils::UriArgs;

        #[test]
        fn test_state_modification() {
            let headers = Arc::new(Mutex::new(HeaderMap::new()));
            headers.lock().unwrap().insert(
                http::header::HOST,
                HeaderValue::from_static("localhost"),
            );

            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/original".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers,
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };

            // Verify initial state
            assert_eq!(state.method, "GET");
            assert_eq!(state.uri_path, "/original");

            // Verify header access
            let guard = state.headers.lock().unwrap();
            assert_eq!(guard.get(http::header::HOST).unwrap(), "localhost");
        }

        #[test]
        fn test_build_uri_roundtrip() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/path".to_string(),
                uri_args: UriArgs(vec![("a".to_string(), "1".to_string())]),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };

            let uri = state.build_uri();
            assert!(uri.starts_with("/path?"));
        }
    }

    // Edge cases
    mod edge_cases {
        use super::*;
        use crate::http::lua::utils::UriArgs;

        #[test]
        fn test_uri_path_with_slashes() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/a/b/c".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };
            assert_eq!(state.build_uri(), "/a/b/c");
        }

        #[test]
        fn test_uri_path_root() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/".to_string(),
                uri_args: UriArgs::new(),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };
            assert_eq!(state.build_uri(), "/");
        }

        #[test]
        fn test_multiple_uri_args_order() {
            let state = CandyReqState {
                method: "GET".to_string(),
                uri_path: "/".to_string(),
                uri_args: UriArgs(vec![
                    ("z".to_string(), "last".to_string()),
                    ("a".to_string(), "first".to_string()),
                    ("m".to_string(), "middle".to_string()),
                ]),
                post_args: None,
                jump: false,
                headers: Arc::new(Mutex::new(HeaderMap::new())),
                redirect_uri: None,
                redirect_status: None,
                output_buffer: String::new(),
            };
            // Should preserve order
            let uri = state.build_uri();
            assert!(uri.find("z=last").is_some());
            assert!(uri.find("a=first").is_some());
            assert!(uri.find("m=middle").is_some());
        }

        #[test]
        fn test_empty_raw_header() {
            let request = CandyRequest {
                uri: Uri::from_static("/"),
                http_version: Some(1.0),
                raw_header: String::new(),
                request_line: "GET / HTTP/1.0".to_string(),
                body: Arc::new(Mutex::new(None)),
            };
            assert!(request.raw_header.is_empty());
            assert!(request.request_line.contains("HTTP/1.0"));
        }

        #[test]
        fn test_http_version_variants() {
            // HTTP 1.0
            let request = CandyRequest {
                uri: Uri::from_static("/"),
                http_version: Some(1.0),
                raw_header: String::new(),
                request_line: "GET / HTTP/1.0".to_string(),
                body: Arc::new(Mutex::new(None)),
            };
            assert_eq!(request.http_version, Some(1.0));

            // HTTP 1.1
            let request = CandyRequest {
                uri: Uri::from_static("/"),
                http_version: Some(1.1),
                raw_header: String::new(),
                request_line: "GET / HTTP/1.1".to_string(),
                body: Arc::new(Mutex::new(None)),
            };
            assert_eq!(request.http_version, Some(1.1));

            // HTTP 2.0
            let request = CandyRequest {
                uri: Uri::from_static("/"),
                http_version: Some(2.0),
                raw_header: String::new(),
                request_line: "GET / HTTP/2".to_string(),
                body: Arc::new(Mutex::new(None)),
            };
            assert_eq!(request.http_version, Some(2.0));

            // None (unknown)
            let request = CandyRequest {
                uri: Uri::from_static("/"),
                http_version: None,
                raw_header: String::new(),
                request_line: "GET /".to_string(),
                body: Arc::new(Mutex::new(None)),
            };
            assert_eq!(request.http_version, None);
        }
    }
}
