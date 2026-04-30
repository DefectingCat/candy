use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, timeout};

use crate::compress::{parse_accept_encoding, gzip_compress, should_compress, CompressionType};
use crate::config::Config;
use crate::http::{Method, ParseError, Parser, Request, Response};
use crate::router::{ResolveResult, get_mime_type, resolve_path};
use crate::socket::create_reuseport_listener;

/// 默认 keep-alive 超时（秒）
const KEEP_ALIVE_TIMEOUT: u64 = 60;

/// Worker 进程主函数
pub fn run(config: &Config) -> std::io::Result<()> {
    println!(
        "Worker {} starting on {}",
        std::process::id(),
        config.server.listen
    );

    // 创建 tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let root = config.server.root.clone();
    let keep_alive_timeout = config.http.keep_alive_timeout;

    runtime.block_on(async {
        // 创建 SO_REUSEPORT listener
        let listener = create_reuseport_listener(config.server.listen)?;

        println!(
            "Worker {} listening on {}",
            std::process::id(),
            config.server.listen
        );

        // 主循环
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let root = root.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, addr, &root, keep_alive_timeout).await
                        {
                            eprintln!("Connection error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    })
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    _addr: std::net::SocketAddr,
    root: &std::path::Path,
    keep_alive_timeout: u64,
) -> std::io::Result<()> {
    let mut buffer = BytesMut::with_capacity(8192);
    let mut parser = Parser::new();
    let mut keep_alive = true;

    while keep_alive {
        // 设置超时读取
        let read_result = timeout(
            Duration::from_secs(keep_alive_timeout),
            read_request(&mut stream, &mut buffer, &mut parser),
        )
        .await;

        match read_result {
            Ok(Ok(request)) => {
                // 检查是否保持连接
                keep_alive = is_keep_alive(&request);

                // 处理请求
                let mut response = handle_request(&request, root);

                // 添加 Connection 头
                if keep_alive {
                    response = response.header("Connection", "keep-alive");
                } else {
                    response = response.header("Connection", "close");
                }

                // 发送响应
                let response_bytes = response.to_bytes();
                stream.write_all(&response_bytes).await?;
            }
            Ok(Err(ParseError::Incomplete)) => {
                // 需要更多数据，继续读取
                continue;
            }
            Ok(Err(_)) => {
                // 解析错误，返回 400
                let response = Response::new(400)
                    .header("Connection", "close")
                    .body(b"Bad Request".to_vec());
                let response_bytes = response.to_bytes();
                stream.write_all(&response_bytes).await?;
                break;
            }
            Err(_) => {
                // 超时，关闭连接
                break;
            }
        }
    }

    Ok(())
}

/// 读取并解析请求
async fn read_request(
    stream: &mut tokio::net::TcpStream,
    buffer: &mut BytesMut,
    parser: &mut Parser,
) -> Result<Request, ParseError> {
    loop {
        // 尝试解析已有数据
        match parser.parse(buffer) {
            Ok((consumed, request)) => {
                // 移除已消费的数据
                let _ = buffer.split_to(consumed);
                return Ok(request);
            }
            Err(ParseError::Incomplete) => {
                // 需要更多数据
                let mut read_buf = [0u8; 4096];
                let n = stream
                    .read(&mut read_buf)
                    .await
                    .map_err(|_| ParseError::Incomplete)?;

                if n == 0 {
                    // 连接关闭
                    return Err(ParseError::Incomplete);
                }

                buffer.extend_from_slice(&read_buf[..n]);
            }
            Err(e) => return Err(e),
        }
    }
}

/// 检查是否保持连接
fn is_keep_alive(request: &Request) -> bool {
    // HTTP/1.1 默认 keep-alive
    let default = request.version == crate::http::HttpVersion::Http11;

    // 检查 Connection 头
    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case(b"Connection") {
            if value.eq_ignore_ascii_case(b"keep-alive") {
                return true;
            }
            if value.eq_ignore_ascii_case(b"close") {
                return false;
            }
        }
    }

    default
}

fn handle_request(request: &Request, root: &std::path::Path) -> Response {
    // 只支持 GET 和 HEAD
    if request.method != Method::GET && request.method != Method::HEAD {
        return Response::new(405)
            .header("Content-Type", "text/plain")
            .body(b"Method Not Allowed".to_vec());
    }

    // 解析路径
    let path_str = String::from_utf8_lossy(&request.path);

    match resolve_path(root, &path_str) {
        ResolveResult::File(file_path) => {
            // 读取文件
            match std::fs::read(&file_path) {
                Ok(content) => {
                    let mime_type = get_mime_type(&file_path);
                    let file_size = content.len();

                    // 检查 Range 头
                    if let Some(range_header) = find_header(&request.headers, b"Range") {
                        return handle_range_request(
                            &range_header,
                            &content,
                            &mime_type,
                            file_size,
                            request.method,
                        );
                    }

                    // 检查压缩协商
                    let (final_content, content_encoding) =
                        if should_compress(&mime_type) {
                            if let Some(accept_encoding) = find_header(&request.headers, b"Accept-Encoding") {
                                let compression = parse_accept_encoding(&accept_encoding);
                                if compression == CompressionType::Gzip {
                                    if let Ok(compressed) = gzip_compress(&content) {
                                        (compressed, Some("gzip"))
                                    } else {
                                        (content, None)
                                    }
                                } else {
                                    (content, None)
                                }
                            } else {
                                (content, None)
                            }
                        } else {
                            (content, None)
                        };

                    // 构建响应
                    let mut response = Response::ok()
                        .header("Content-Type", mime_type)
                        .header("Accept-Ranges", "bytes")
                        .header("Vary", "Accept-Encoding");

                    // 添加 Content-Encoding 头
                    if let Some(encoding) = content_encoding {
                        response = response.header("Content-Encoding", encoding);
                    }

                    // HEAD 请求不返回 body
                    if request.method == Method::HEAD {
                        response = response.header("Content-Length", final_content.len().to_string());
                    } else {
                        response = response.body(final_content);
                    }

                    response
                }
                Err(_) => Response::internal_error(),
            }
        }
        ResolveResult::Directory(_) => {
            // 目录没有 index.html
            Response::forbidden()
        }
        ResolveResult::NotFound => Response::not_found(),
        ResolveResult::Forbidden => Response::forbidden(),
    }
}

/// 查找指定头部
fn find_header(headers: &[(bytes::Bytes, bytes::Bytes)], name: &[u8]) -> Option<Vec<u8>> {
    for (n, v) in headers {
        if n.eq_ignore_ascii_case(name) {
            return Some(v.to_vec());
        }
    }
    None
}

/// 解析 Range 头，格式: bytes=start-end 或 bytes=start-
pub fn parse_range_header(range_header: &[u8], file_size: usize) -> Option<(usize, usize)> {
    let range_str = std::str::from_utf8(range_header).ok()?;

    // 必须以 "bytes=" 开头
    if !range_str.starts_with("bytes=") {
        return None;
    }

    let range_spec = &range_str[6..]; // 跳过 "bytes="

    // 解析 start-end 或 start-
    let parts: Vec<&str> = range_spec.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let start: usize = if parts[0].is_empty() {
        // -end 格式：最后 end 个字节
        let end: usize = parts[1].trim().parse().ok()?;
        if end > file_size {
            return None;
        }
        return Some((file_size - end, file_size - 1));
    } else {
        parts[0].trim().parse().ok()?
    };

    let end: usize = if parts[1].is_empty() {
        // start- 格式：从 start 到文件末尾
        file_size - 1
    } else {
        let end: usize = parts[1].trim().parse().ok()?;
        end.min(file_size - 1)
    };

    // 验证范围有效性
    if start > end || start >= file_size {
        return None;
    }

    Some((start, end))
}

/// 处理 Range 请求
fn handle_range_request(
    range_header: &[u8],
    content: &[u8],
    mime_type: &str,
    file_size: usize,
    method: Method,
) -> Response {
    match parse_range_header(range_header, file_size) {
        Some((start, end)) => {
            let range_length = end - start + 1;
            let content_range = format!("bytes {}-{}/{}", start, end, file_size);

            let mut response = Response::partial_content()
                .header("Content-Type", mime_type)
                .header("Content-Range", content_range)
                .header("Accept-Ranges", "bytes");

            // HEAD 请求不返回 body
            if method == Method::HEAD {
                response = response.header("Content-Length", range_length.to_string());
            } else {
                response = response.body(content[start..=end].to_vec());
            }

            response
        }
        None => {
            // 无效的 Range 请求
            Response::range_not_satisfiable(&format!("bytes */{}", file_size))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{HttpVersion, Request};

    #[test]
    fn test_parse_range_header_start_end() {
        let result = parse_range_header(b"bytes=0-99", 1000);
        assert_eq!(result, Some((0, 99)));
    }

    #[test]
    fn test_parse_range_header_start_only() {
        let result = parse_range_header(b"bytes=500-", 1000);
        assert_eq!(result, Some((500, 999)));
    }

    #[test]
    fn test_parse_range_header_suffix() {
        let result = parse_range_header(b"bytes=-100", 1000);
        assert_eq!(result, Some((900, 999)));
    }

    #[test]
    fn test_parse_range_header_invalid_start() {
        let result = parse_range_header(b"bytes=1000-2000", 1000);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_range_header_invalid_format() {
        let result = parse_range_header(b"invalid", 1000);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_range_header_no_bytes_prefix() {
        let result = parse_range_header(b"chunks=0-99", 1000);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_header() {
        let headers = vec![
            (bytes::Bytes::from("Host"), bytes::Bytes::from("localhost")),
            (
                bytes::Bytes::from("Range"),
                bytes::Bytes::from("bytes=0-99"),
            ),
        ];
        let result = find_header(&headers, b"Range");
        assert_eq!(result, Some(b"bytes=0-99".to_vec()));
    }

    #[test]
    fn test_find_header_not_found() {
        let headers = vec![(bytes::Bytes::from("Host"), bytes::Bytes::from("localhost"))];
        let result = find_header(&headers, b"Range");
        assert_eq!(result, None);
    }

    #[test]
    fn test_is_keep_alive() {
        let request = Request {
            method: Method::GET,
            path: bytes::Bytes::from("/"),
            version: HttpVersion::Http11,
            headers: vec![],
            body: None,
        };
        assert!(is_keep_alive(&request));
    }

    #[test]
    fn test_is_keep_alive_http10_default() {
        let request = Request {
            method: Method::GET,
            path: bytes::Bytes::from("/"),
            version: HttpVersion::Http10,
            headers: vec![],
            body: None,
        };
        assert!(!is_keep_alive(&request));
    }

    #[test]
    fn test_is_keep_alive_with_header() {
        let request = Request {
            method: Method::GET,
            path: bytes::Bytes::from("/"),
            version: HttpVersion::Http10,
            headers: vec![(
                bytes::Bytes::from("Connection"),
                bytes::Bytes::from("keep-alive"),
            )],
            body: None,
        };
        assert!(is_keep_alive(&request));
    }

    #[test]
    fn test_is_keep_alive_close_header() {
        let request = Request {
            method: Method::GET,
            path: bytes::Bytes::from("/"),
            version: HttpVersion::Http11,
            headers: vec![(
                bytes::Bytes::from("Connection"),
                bytes::Bytes::from("close"),
            )],
            body: None,
        };
        assert!(!is_keep_alive(&request));
    }
}
