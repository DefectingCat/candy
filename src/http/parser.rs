use bytes::Bytes;

/// HTTP 请求方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum Method {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    OPTIONS,
    CONNECT,
    TRACE,
    PATCH,
}

impl Method {
    /// 从字节解析方法
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"GET" => Some(Method::GET),
            b"HEAD" => Some(Method::HEAD),
            b"POST" => Some(Method::POST),
            b"PUT" => Some(Method::PUT),
            b"DELETE" => Some(Method::DELETE),
            b"OPTIONS" => Some(Method::OPTIONS),
            b"CONNECT" => Some(Method::CONNECT),
            b"TRACE" => Some(Method::TRACE),
            b"PATCH" => Some(Method::PATCH),
            _ => None,
        }
    }
}

/// HTTP 版本
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http10,
    Http11,
}

/// HTTP 请求
#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub path: Bytes,
    pub version: HttpVersion,
    pub headers: Vec<(Bytes, Bytes)>,
    pub body: Option<Bytes>,
}

/// 解析错误
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Incomplete request")]
    Incomplete,

    #[error("Invalid method")]
    InvalidMethod,

    #[error("Invalid HTTP version")]
    InvalidVersion,

    #[error("Invalid request line")]
    InvalidRequestLine,

    #[error("Invalid header")]
    InvalidHeader,

    #[error("Request headers too large")]
    TooLarge,

    #[error("Request body too large")]
    BodyTooLarge,
}

/// HTTP 请求解析器
pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    /// 解析请求，返回 (consumed_bytes, result)
    /// max_header_size: 最大请求头大小（字节），超过返回 ParseError::TooLarge
    /// max_body_size: 最大请求体大小（字节），超过返回 ParseError::BodyTooLarge
    pub fn parse(
        &mut self,
        buffer: &[u8],
        max_header_size: usize,
        max_body_size: usize,
    ) -> Result<(usize, Request), ParseError> {
        // 直接查找 \r\n\r\n 作为头部结束
        let headers_end = find_headers_end(buffer, 0)?;

        // 检查头部大小（headers_end 是 \r\n\r\n 开始的位置，即头部结束位置）
        if headers_end > max_header_size {
            return Err(ParseError::TooLarge);
        }

        // 查找请求行结束（第一个 \r\n）
        let line_end = find_first_crlf(buffer)?;

        // 解析请求行
        let (method, path, version) = parse_request_line(&buffer[..line_end])?;

        // 头部开始位置（请求行后）
        let header_start = line_end + 2;

        // 解析头部（从请求行后到 \r\n\r\n 前）
        let headers = if header_start < headers_end {
            parse_headers(&buffer[header_start..headers_end])?
        } else {
            Vec::new()
        };

        // 查找 Content-Length
        let content_length = find_content_length(&headers);

        // 检查请求体大小
        if let Some(len) = content_length
            && len > max_body_size
        {
            return Err(ParseError::BodyTooLarge);
        }

        // 计算总消耗字节数
        let headers_total = headers_end + 4;
        let body_start = headers_total;

        // 检查是否有足够的 body 数据
        let body = if let Some(len) = content_length {
            if buffer.len() < body_start + len {
                return Err(ParseError::Incomplete);
            }
            Some(Bytes::copy_from_slice(
                &buffer[body_start..body_start + len],
            ))
        } else {
            None
        };

        let consumed = body_start + content_length.unwrap_or(0);

        Ok((
            consumed,
            Request {
                method,
                path: Bytes::copy_from_slice(path),
                version,
                headers: headers
                    .into_iter()
                    .map(|(k, v)| (Bytes::copy_from_slice(k), Bytes::copy_from_slice(v)))
                    .collect(),
                body,
            },
        ))
    }
}

/// 查找 Content-Length 头部值
fn find_content_length(headers: &[(&[u8], &[u8])]) -> Option<usize> {
    for (name, value) in headers {
        if name.eq_ignore_ascii_case(b"Content-Length") {
            let s = std::str::from_utf8(value).ok()?;
            return s.parse::<usize>().ok();
        }
    }
    None
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// 查找第一个 \r\n
fn find_first_crlf(buffer: &[u8]) -> Result<usize, ParseError> {
    for i in 0..buffer.len().saturating_sub(1) {
        if buffer[i] == b'\r' && buffer[i + 1] == b'\n' {
            return Ok(i);
        }
    }
    Err(ParseError::Incomplete)
}

/// 查找头部结束 (\r\n\r\n)
fn find_headers_end(buffer: &[u8], _start: usize) -> Result<usize, ParseError> {
    let mut pos = 0;
    while pos + 3 < buffer.len() {
        if buffer[pos] == b'\r'
            && buffer[pos + 1] == b'\n'
            && buffer[pos + 2] == b'\r'
            && buffer[pos + 3] == b'\n'
        {
            return Ok(pos);
        }
        pos += 1;
    }
    Err(ParseError::Incomplete)
}

/// 解析请求行: METHOD PATH VERSION
fn parse_request_line(line: &[u8]) -> Result<(Method, &[u8], HttpVersion), ParseError> {
    let parts: Vec<&[u8]> = line.split(|&c| c == b' ').collect();

    if parts.len() != 3 {
        return Err(ParseError::InvalidRequestLine);
    }

    let method = Method::from_bytes(parts[0]).ok_or(ParseError::InvalidMethod)?;

    let version = match parts[2] {
        b"HTTP/1.0" => HttpVersion::Http10,
        b"HTTP/1.1" => HttpVersion::Http11,
        _ => return Err(ParseError::InvalidVersion),
    };

    Ok((method, parts[1], version))
}

/// 解析头部
type HeadersResult<'a> = Result<Vec<(&'a [u8], &'a [u8])>, ParseError>;

fn parse_headers(data: &[u8]) -> HeadersResult<'_> {
    let mut headers = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // 查找行结束
        let line_end = if let Some(end) = find_crlf(&data[pos..]) {
            pos + end
        } else {
            // 最后一行可能没有 \r\n
            if pos < data.len() {
                data.len()
            } else {
                break;
            }
        };

        // 空行表示头部结束
        if line_end == pos {
            break;
        }

        // 解析 header: value
        let line = &data[pos..line_end];
        if let Some(colon_pos) = line.iter().position(|&c| c == b':') {
            let name = &line[..colon_pos];
            let value = &line[colon_pos + 1..];
            // 去除前导空格
            let value_start = value.iter().position(|&c| c != b' ').unwrap_or(value.len());
            headers.push((name, &value[value_start..]));
        } else {
            return Err(ParseError::InvalidHeader);
        }

        pos = line_end + 2; // skip \r\n (if present)
        // 如果已经到达末尾，退出
        if pos > data.len() {
            break;
        }
    }

    Ok(headers)
}

fn find_crlf(data: &[u8]) -> Option<usize> {
    (0..data.len().saturating_sub(1)).find(|&i| data[i] == b'\r' && data[i + 1] == b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_MAX_HEADER_SIZE: usize = 8192;
    const DEFAULT_MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB

    #[test]
    fn test_parse_get_request() {
        let raw = b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut parser = Parser::new();
        let (consumed, req) = parser
            .parse(raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE)
            .unwrap();

        assert_eq!(consumed, raw.len());
        assert_eq!(req.method, Method::GET);
        assert_eq!(&req.path[..], b"/index.html");
        assert_eq!(req.version, HttpVersion::Http11);
        assert_eq!(req.headers.len(), 1);
        assert_eq!(&req.headers[0].0[..], b"Host");
        assert_eq!(&req.headers[0].1[..], b"localhost");
    }

    #[test]
    fn test_parse_head_request() {
        let raw = b"HEAD /test HTTP/1.0\r\n\r\n";
        let mut parser = Parser::new();
        let (_, req) = parser
            .parse(raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE)
            .unwrap();

        assert_eq!(req.method, Method::HEAD);
        assert_eq!(req.version, HttpVersion::Http10);
    }

    #[test]
    fn test_parse_invalid_method() {
        let raw = b"INVALID / HTTP/1.1\r\n\r\n";
        let mut parser = Parser::new();
        assert!(
            parser
                .parse(raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE)
                .is_err()
        );
    }

    #[test]
    fn test_parse_incomplete() {
        let raw = b"GET / HTTP/1.1\r\n";
        let mut parser = Parser::new();
        assert!(matches!(
            parser.parse(raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE),
            Err(ParseError::Incomplete)
        ));
    }

    #[test]
    fn test_parse_post_with_body() {
        let raw = b"POST /submit HTTP/1.1\r\nContent-Length: 11\r\n\r\nHello World";
        let mut parser = Parser::new();
        let (consumed, req) = parser
            .parse(raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE)
            .unwrap();

        assert_eq!(consumed, raw.len());
        assert_eq!(req.method, Method::POST);
        assert!(req.body.is_some());
        assert_eq!(&req.body.unwrap()[..], b"Hello World");
    }

    #[test]
    fn test_parse_all_methods() {
        let methods: &[(&[u8], Method)] = &[
            (b"GET / HTTP/1.1\r\n\r\n", Method::GET),
            (b"HEAD / HTTP/1.1\r\n\r\n", Method::HEAD),
            (b"POST / HTTP/1.1\r\n\r\n", Method::POST),
            (b"PUT / HTTP/1.1\r\n\r\n", Method::PUT),
            (b"DELETE / HTTP/1.1\r\n\r\n", Method::DELETE),
            (b"OPTIONS / HTTP/1.1\r\n\r\n", Method::OPTIONS),
            (b"CONNECT / HTTP/1.1\r\n\r\n", Method::CONNECT),
            (b"TRACE / HTTP/1.1\r\n\r\n", Method::TRACE),
            (b"PATCH / HTTP/1.1\r\n\r\n", Method::PATCH),
        ];

        let mut parser = Parser::new();
        for (raw, expected_method) in methods {
            let (_, req) = parser
                .parse(raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE)
                .unwrap();
            assert_eq!(req.method, *expected_method);
        }
    }

    #[test]
    fn test_parse_large_headers_within_limit() {
        // 构造一个刚好在限制内的头部（约 6KB）
        let mut raw = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..200 {
            raw.extend_from_slice(format!("X-Custom-Header-{}: value{}\r\n", i, i).as_bytes());
        }
        raw.extend_from_slice(b"\r\n");

        let mut parser = Parser::new();
        let (_, req) = parser
            .parse(&raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE)
            .unwrap();
        assert_eq!(req.headers.len(), 200);
    }

    #[test]
    fn test_parse_headers_exceed_limit() {
        // 构造一个超过 8KB 的头部
        let mut raw = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..500 {
            raw.extend_from_slice(format!("X-Custom-Header-{}: value{}\r\n", i, i).as_bytes());
        }
        raw.extend_from_slice(b"\r\n");

        let mut parser = Parser::new();
        let result = parser.parse(&raw, DEFAULT_MAX_HEADER_SIZE, DEFAULT_MAX_BODY_SIZE);
        assert!(matches!(result, Err(ParseError::TooLarge)));
    }

    #[test]
    fn test_parse_headers_exact_limit() {
        // 构造一个刚好等于限制的头部
        let raw = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let header_size = raw.len() - 4; // 减去 body 分隔符

        let mut parser = Parser::new();
        let result = parser.parse(raw, header_size, DEFAULT_MAX_BODY_SIZE);
        // 刚好等于限制应该通过
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_body_within_limit() {
        // 构造一个请求体在限制内的请求
        let body = "x".repeat(1000);
        let raw = format!(
            "POST /upload HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let mut parser = Parser::new();
        let result = parser.parse(raw.as_bytes(), DEFAULT_MAX_HEADER_SIZE, 2000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_body_exceed_limit() {
        // 构造一个请求体超过限制的请求
        let body = "x".repeat(1000);
        let raw = format!(
            "POST /upload HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let mut parser = Parser::new();
        let result = parser.parse(raw.as_bytes(), DEFAULT_MAX_HEADER_SIZE, 500);
        assert!(matches!(result, Err(ParseError::BodyTooLarge)));
    }
}
