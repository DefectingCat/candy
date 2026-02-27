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
        redirect_status: None,
        output_buffer: String::new(),
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

    // 检查请求状态中的输出缓冲区（来自 print 调用）
    let output_buffer = {
        let state = ctx
            .req_state
            .lock()
            .map_err(|_| RouteError::InternalError())?;
        state.output_buffer.clone()
    };

    // 合并原始响应体和 print 输出
    let final_body = format!("{}{}", res.body, output_buffer);

    let mut response = Response::builder();
    let body = Body::from(final_body);
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

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderMap, HeaderValue, header};

    // Helper functions for tests
    fn http_version_to_string(version: http::Version) -> &'static str {
        match version {
            http::Version::HTTP_09 => "HTTP/0.9",
            http::Version::HTTP_10 => "HTTP/1.0",
            http::Version::HTTP_11 => "HTTP/1.1",
            http::Version::HTTP_2 => "HTTP/2.0",
            http::Version::HTTP_3 => "HTTP/3.0",
            _ => "HTTP/1.1",
        }
    }

    fn http_version_to_float(version: http::Version) -> Option<f32> {
        match version {
            http::Version::HTTP_09 => Some(0.9),
            http::Version::HTTP_10 => Some(1.0),
            http::Version::HTTP_11 => Some(1.1),
            http::Version::HTTP_2 => Some(2.0),
            http::Version::HTTP_3 => Some(3.0),
            _ => None,
        }
    }

    fn build_raw_header(headers: &http::HeaderMap) -> String {
        let mut headers_str = String::new();
        for (name, value) in headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_str.push_str(&format!("{}: {}\r\n", name, v));
            }
        }
        headers_str
    }

    fn build_request_line(
        method: &str,
        uri_pq: Option<&http::uri::PathAndQuery>,
        version_str: &str,
    ) -> String {
        format!(
            "{} {} {}",
            method,
            uri_pq.map(|pq| pq.as_str()).unwrap_or("/"),
            version_str
        )
    }

    fn parse_uri_args(uri: &http::Uri) -> (String, UriArgs) {
        uri.path_and_query()
            .map(|pq| {
                let (path, query) = pq.as_str().split_once('?').unwrap_or((pq.as_str(), ""));
                (path.to_string(), UriArgs::from_query(query))
            })
            .unwrap_or_else(|| ("/".to_string(), UriArgs::new()))
    }

    // http_version_to_string tests
    mod http_version_to_string {
        use super::*;

        #[test]
        fn test_http_09() {
            assert_eq!(http_version_to_string(http::Version::HTTP_09), "HTTP/0.9");
        }

        #[test]
        fn test_http_10() {
            assert_eq!(http_version_to_string(http::Version::HTTP_10), "HTTP/1.0");
        }

        #[test]
        fn test_http_11() {
            assert_eq!(http_version_to_string(http::Version::HTTP_11), "HTTP/1.1");
        }

        #[test]
        fn test_http_2() {
            assert_eq!(http_version_to_string(http::Version::HTTP_2), "HTTP/2.0");
        }

        #[test]
        fn test_http_3() {
            assert_eq!(http_version_to_string(http::Version::HTTP_3), "HTTP/3.0");
        }

        #[test]
        fn test_all_known_versions() {
            // All known HTTP versions should return proper string
            assert_eq!(http_version_to_string(http::Version::HTTP_09), "HTTP/0.9");
            assert_eq!(http_version_to_string(http::Version::HTTP_10), "HTTP/1.0");
            assert_eq!(http_version_to_string(http::Version::HTTP_11), "HTTP/1.1");
            assert_eq!(http_version_to_string(http::Version::HTTP_2), "HTTP/2.0");
            assert_eq!(http_version_to_string(http::Version::HTTP_3), "HTTP/3.0");
        }
    }

    // http_version_to_float tests
    mod http_version_to_float {
        use super::*;

        #[test]
        fn test_http_09() {
            assert_eq!(http_version_to_float(http::Version::HTTP_09), Some(0.9));
        }

        #[test]
        fn test_http_10() {
            assert_eq!(http_version_to_float(http::Version::HTTP_10), Some(1.0));
        }

        #[test]
        fn test_http_11() {
            assert_eq!(http_version_to_float(http::Version::HTTP_11), Some(1.1));
        }

        #[test]
        fn test_http_2() {
            assert_eq!(http_version_to_float(http::Version::HTTP_2), Some(2.0));
        }

        #[test]
        fn test_http_3() {
            assert_eq!(http_version_to_float(http::Version::HTTP_3), Some(3.0));
        }

        #[test]
        fn test_all_known_versions_float() {
            // All known HTTP versions should return Some
            assert_eq!(http_version_to_float(http::Version::HTTP_09), Some(0.9));
            assert_eq!(http_version_to_float(http::Version::HTTP_10), Some(1.0));
            assert_eq!(http_version_to_float(http::Version::HTTP_11), Some(1.1));
            assert_eq!(http_version_to_float(http::Version::HTTP_2), Some(2.0));
            assert_eq!(http_version_to_float(http::Version::HTTP_3), Some(3.0));
        }
    }

    // build_raw_header tests
    mod build_raw_header {
        use super::*;

        #[test]
        fn test_empty_headers() {
            let headers = HeaderMap::new();
            let result = build_raw_header(&headers);
            assert_eq!(result, "");
        }

        #[test]
        fn test_single_header() {
            let mut headers = HeaderMap::new();
            headers.insert(header::HOST, HeaderValue::from_static("localhost"));
            let result = build_raw_header(&headers);
            assert!(result.contains("host: localhost"));
        }

        #[test]
        fn test_multiple_headers() {
            let mut headers = HeaderMap::new();
            headers.insert(header::HOST, HeaderValue::from_static("localhost"));
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            let result = build_raw_header(&headers);
            assert!(result.contains("host: localhost"));
            assert!(result.contains("content-type: application/json"));
        }

        #[test]
        fn test_header_format() {
            let mut headers = HeaderMap::new();
            headers.insert(header::ACCEPT, HeaderValue::from_static("text/html"));
            let result = build_raw_header(&headers);
            assert!(result.ends_with("\r\n"));
            assert!(result.contains(": "));
        }

        #[test]
        fn test_header_value_with_special_chars() {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::ACCEPT,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            let result = build_raw_header(&headers);
            assert!(result.contains("text/html; charset=utf-8"));
        }
    }

    // build_request_line tests
    mod build_request_line {
        use super::*;

        #[test]
        fn test_simple_get() {
            let uri = Uri::from_static("/");
            let pq = uri.path_and_query();
            let result = build_request_line("GET", pq, "HTTP/1.1");
            assert_eq!(result, "GET / HTTP/1.1");
        }

        #[test]
        fn test_with_path() {
            let uri = Uri::from_static("/api/users");
            let pq = uri.path_and_query();
            let result = build_request_line("POST", pq, "HTTP/1.1");
            assert_eq!(result, "POST /api/users HTTP/1.1");
        }

        #[test]
        fn test_with_query_string() {
            let uri = Uri::from_static("/search?q=test");
            let pq = uri.path_and_query();
            let result = build_request_line("GET", pq, "HTTP/1.1");
            assert_eq!(result, "GET /search?q=test HTTP/1.1");
        }

        #[test]
        fn test_with_path_and_query() {
            let uri = Uri::from_static("/api/item/123?fields=name,email");
            let pq = uri.path_and_query();
            let result = build_request_line("PUT", pq, "HTTP/1.1");
            assert_eq!(result, "PUT /api/item/123?fields=name,email HTTP/1.1");
        }

        #[test]
        fn test_none_uri() {
            let result = build_request_line("GET", None, "HTTP/1.1");
            assert_eq!(result, "GET / HTTP/1.1");
        }

        #[test]
        fn test_http_10() {
            let uri = Uri::from_static("/");
            let pq = uri.path_and_query();
            let result = build_request_line("GET", pq, "HTTP/1.0");
            assert_eq!(result, "GET / HTTP/1.0");
        }

        #[test]
        fn test_delete_method() {
            let uri = Uri::from_static("/resource/1");
            let pq = uri.path_and_query();
            let result = build_request_line("DELETE", pq, "HTTP/1.1");
            assert_eq!(result, "DELETE /resource/1 HTTP/1.1");
        }
    }

    // parse_uri_args tests
    mod parse_uri_args {
        use super::*;

        #[test]
        fn test_root_path() {
            let uri = Uri::from_static("/");
            let (path, args) = parse_uri_args(&uri);
            assert_eq!(path, "/");
            assert!(args.0.is_empty());
        }

        #[test]
        fn test_simple_path() {
            let uri = Uri::from_static("/api/users");
            let (path, args) = parse_uri_args(&uri);
            assert_eq!(path, "/api/users");
            assert!(args.0.is_empty());
        }

        #[test]
        fn test_path_with_query() {
            let uri = Uri::from_static("/search?q=rust");
            let (path, args) = parse_uri_args(&uri);
            assert_eq!(path, "/search");
            assert_eq!(args.0.len(), 1);
            assert_eq!(args.0[0], ("q".to_string(), "rust".to_string()));
        }

        #[test]
        fn test_path_with_multiple_params() {
            let uri = Uri::from_static("/api?q=1&page=2&size=10");
            let (path, args) = parse_uri_args(&uri);
            assert_eq!(path, "/api");
            assert_eq!(args.0.len(), 3);
        }

        #[test]
        fn test_empty_query() {
            let uri = Uri::from_static("/api?");
            let (path, args) = parse_uri_args(&uri);
            assert_eq!(path, "/api");
            assert!(args.0.is_empty());
        }

        #[test]
        fn test_no_query() {
            let uri = Uri::from_static("/api/users/123");
            let (path, args) = parse_uri_args(&uri);
            assert_eq!(path, "/api/users/123");
            assert!(args.0.is_empty());
        }

        #[test]
        fn test_special_characters_in_query() {
            let uri = Uri::from_static("/search?q=hello%20world");
            let (path, args) = parse_uri_args(&uri);
            assert_eq!(path, "/search");
            assert_eq!(args.0[0].0, "q");
        }

        #[test]
        fn test_nested_path() {
            let uri = Uri::from_static("/a/b/c/d");
            let (path, _args) = parse_uri_args(&uri);
            assert_eq!(path, "/a/b/c/d");
        }

        #[test]
        fn test_trailing_slash() {
            let uri = Uri::from_static("/api/");
            let (path, _args) = parse_uri_args(&uri);
            assert_eq!(path, "/api/");
        }
    }

    // Integration tests
    mod integration {
        use super::*;

        #[test]
        fn test_full_request_line_flow() {
            // Test complete flow: version -> string -> request line
            let version = http::Version::HTTP_11;
            let version_str = http_version_to_string(version);
            let uri = Uri::from_static("/test?a=1");
            let request_line = build_request_line("GET", uri.path_and_query(), version_str);

            assert_eq!(request_line, "GET /test?a=1 HTTP/1.1");
        }

        #[test]
        fn test_full_header_flow() {
            let mut headers = HeaderMap::new();
            headers.insert(header::HOST, HeaderValue::from_static("example.com"));
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );

            let raw = build_raw_header(&headers);

            assert!(raw.contains("host: example.com"));
            assert!(raw.contains("content-type: application/json"));
        }

        #[test]
        fn test_uri_to_state_flow() {
            let uri = Uri::from_static("/api/users?id=42&name=test");
            let (path, uri_args) = parse_uri_args(&uri);

            // Build URI back
            let built_uri = if uri_args.0.is_empty() {
                path.clone()
            } else {
                format!("{}?{}", path, uri_args.to_query())
            };

            assert!(built_uri.contains("/api/users"));
            assert!(built_uri.contains("id=42"));
            assert!(built_uri.contains("name=test"));
        }

        #[test]
        fn test_version_conversion_consistency() {
            // Verify float and string conversions are consistent
            let versions = [
                http::Version::HTTP_09,
                http::Version::HTTP_10,
                http::Version::HTTP_11,
                http::Version::HTTP_2,
                http::Version::HTTP_3,
            ];

            for v in versions {
                let float_val = http_version_to_float(v);
                let string_val = http_version_to_string(v);

                match float_val {
                    Some(0.9) => assert_eq!(string_val, "HTTP/0.9"),
                    Some(1.0) => assert_eq!(string_val, "HTTP/1.0"),
                    Some(1.1) => assert_eq!(string_val, "HTTP/1.1"),
                    Some(2.0) => assert_eq!(string_val, "HTTP/2.0"),
                    Some(3.0) => assert_eq!(string_val, "HTTP/3.0"),
                    _ => {}
                }
            }
        }
    }

    // Edge cases
    mod edge_cases {
        use super::*;

        #[test]
        fn test_raw_header_with_binary_value() {
            // Binary values that can't be converted to &str should be skipped
            let mut headers = HeaderMap::new();
            // Content-Disposition might have binary data in real scenarios
            headers.insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"test.txt\""),
            );
            let result = build_raw_header(&headers);
            assert!(result.contains("content-disposition"));
        }

        #[test]
        fn test_request_line_with_complex_path() {
            let uri = Uri::from_static("/api/v1/users/123/profile/settings?debug=true");
            let pq = uri.path_and_query();
            let result = build_request_line("PATCH", pq, "HTTP/2.0");

            assert!(result.starts_with("PATCH "));
            assert!(result.ends_with(" HTTP/2.0"));
            assert!(result.contains("/api/v1/users/123/profile/settings"));
        }

        #[test]
        fn test_uri_args_preserves_order() {
            let uri = Uri::from_static("/?a=1&b=2&c=3&d=4&e=5");
            let (_, args) = parse_uri_args(&uri);

            let keys: Vec<_> = args.0.iter().map(|(k, _)| k.clone()).collect();
            assert_eq!(keys, vec!["a", "b", "c", "d", "e"]);
        }

        #[test]
        fn test_root_path_parsing() {
            // Root path "/" should parse correctly
            let uri = Uri::from_static("/");
            let (path, _) = parse_uri_args(&uri);
            assert_eq!(path, "/");
        }
    }
}
