use bytes::BytesMut;

/// HTTP 响应
pub struct Response {
    pub status: u16,
    pub status_text: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl Response {
    /// 创建新响应
    pub fn new(status: u16) -> Self {
        let status_text = status_text(status);
        Response {
            status,
            status_text,
            headers: Vec::new(),
            body: None,
        }
    }

    /// 添加头部
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// 设置 body
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();

        // 状态行
        let status_line = format!("HTTP/1.1 {} {}\r\n", self.status, self.status_text);
        buf.extend_from_slice(status_line.as_bytes());

        // 头部
        for (name, value) in &self.headers {
            buf.extend_from_slice(format!("{}: {}\r\n", name, value).as_bytes());
        }

        // Content-Length
        if let Some(ref body) = self.body {
            buf.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
        }

        // 空行
        buf.extend_from_slice(b"\r\n");

        // Body
        if let Some(ref body) = self.body {
            buf.extend_from_slice(body);
        }

        buf
    }

    /// 200 OK
    pub fn ok() -> Self {
        Self::new(200)
    }

    /// 404 Not Found
    pub fn not_found() -> Self {
        Self::new(404).body(b"Not Found".to_vec())
    }

    /// 403 Forbidden
    pub fn forbidden() -> Self {
        Self::new(403).body(b"Forbidden".to_vec())
    }

    /// 500 Internal Server Error
    pub fn internal_error() -> Self {
        Self::new(500).body(b"Internal Server Error".to_vec())
    }

    /// 206 Partial Content
    pub fn partial_content() -> Self {
        Self::new(206)
    }

    /// 416 Range Not Satisfiable
    pub fn range_not_satisfiable(content_range: &str) -> Self {
        Self::new(416)
            .header("Content-Range", content_range)
            .body(b"Range Not Satisfiable".to_vec())
    }

    /// 获取状态码
    pub fn status_code(&self) -> u16 {
        self.status
    }

    /// 获取 Content-Type
    pub fn content_type(&self) -> &str {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("Content-Type"))
            .map(|(_, value)| value.as_str())
            .unwrap_or("application/octet-stream")
    }

    /// 获取 body 长度
    pub fn body_len(&self) -> usize {
        self.body.as_ref().map(|b| b.len()).unwrap_or(0)
    }

    /// 消费 self 返回 body
    pub fn into_body(self) -> Vec<u8> {
        self.body.unwrap_or_default()
    }
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        416 => "Range Not Satisfiable",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_to_bytes() {
        let response = Response::ok()
            .header("Content-Type", "text/html")
            .body(b"<h1>Hello</h1>".to_vec());

        let bytes = response.to_bytes();
        let s = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(s.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(s.contains("Content-Type: text/html\r\n"));
        assert!(s.contains("Content-Length: 14\r\n"));
        assert!(s.ends_with("<h1>Hello</h1>"));
    }

    #[test]
    fn test_not_found_response() {
        let response = Response::not_found();
        assert_eq!(response.status, 404);
        assert!(response.body.is_some());
    }
}
