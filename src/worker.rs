use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

use crate::config::Config;
use crate::http::{Method, ParseError, Parser, Request, Response};
use crate::router::{get_mime_type, resolve_path, ResolveResult};
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
                        if let Err(e) = handle_connection(stream, addr, &root, keep_alive_timeout).await {
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
                    let mut response = Response::ok()
                        .header("Content-Type", mime_type);

                    // HEAD 请求不返回 body
                    if request.method == Method::HEAD {
                        response = response.header("Content-Length", content.len().to_string());
                    } else {
                        response = response.body(content);
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
