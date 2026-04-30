use bytes::Bytes;

/// HTTP 请求方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    #[error("Request too large")]
    TooLarge,
}

/// HTTP 请求解析器（状态机）
pub struct Parser {
    state: ParseState,
}

#[derive(Debug, Clone, Copy)]
enum ParseState {
    Start,
    Method,
    Path,
    Version,
    HeaderName,
    HeaderValue,
    HeadersEnd,
    Complete,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            state: ParseState::Start,
        }
    }

    /// 解析请求，返回 (consumed_bytes, result)
    pub fn parse(&mut self, buffer: &[u8]) -> Result<(usize, Request), ParseError> {
        // 直接查找 \r\n\r\n 作为请求结束
        let headers_end = find_headers_end(buffer, 0)?;

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

        let consumed = headers_end + 4; // +4 for \r\n\r\n

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
            },
        ))
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// 查找行结束 (\r\n)
fn find_line_end(buffer: &[u8], pos: &mut usize) -> Result<usize, ParseError> {
    while *pos < buffer.len() - 1 {
        if buffer[*pos] == b'\r' && buffer[*pos + 1] == b'\n' {
            let end = *pos;
            *pos += 2;
            return Ok(end);
        }
        *pos += 1;
    }
    Err(ParseError::Incomplete)
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
fn parse_headers(data: &[u8]) -> Result<Vec<(&[u8], &[u8])>, ParseError> {
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
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == b'\r' && data[i + 1] == b'\n' {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_get_request() {
        let raw = b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut parser = Parser::new();
        let (consumed, req) = parser.parse(raw).unwrap();

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
        let (_, req) = parser.parse(raw).unwrap();

        assert_eq!(req.method, Method::HEAD);
        assert_eq!(req.version, HttpVersion::Http10);
    }

    #[test]
    fn test_parse_invalid_method() {
        let raw = b"INVALID / HTTP/1.1\r\n\r\n";
        let mut parser = Parser::new();
        assert!(parser.parse(raw).is_err());
    }

    #[test]
    fn test_parse_incomplete() {
        let raw = b"GET / HTTP/1.1\r\n";
        let mut parser = Parser::new();
        assert!(matches!(parser.parse(raw), Err(ParseError::Incomplete)));
    }
}
