use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::Config;
use crate::http::{Method, ParseError, Parser, Response};
use crate::router::{get_mime_type, resolve_path, ResolveResult};
use crate::socket::create_reuseport_listener;

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
                        if let Err(e) = handle_connection(stream, addr, &root).await {
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
    addr: std::net::SocketAddr,
    root: &std::path::Path,
) -> std::io::Result<()> {
    // 读取请求
    let mut buffer = BytesMut::with_capacity(8192);
    let mut parser = Parser::new();

    loop {
        // 读取数据
        let mut read_buf = [0u8; 4096];
        let n = stream.read(&mut read_buf).await?;

        if n == 0 {
            // 连接关闭
            return Ok(());
        }

        buffer.extend_from_slice(&read_buf[..n]);

        // 尝试解析请求
        match parser.parse(&buffer) {
            Ok((consumed, request)) => {
                // 移除已消费的数据
                let _ = buffer.split_to(consumed);

                // 处理请求
                let response = handle_request(&request, root);

                // 发送响应
                let response_bytes = response.to_bytes();
                stream.write_all(&response_bytes).await?;

                // 只处理一个请求（暂不支持 keep-alive）
                return Ok(());
            }
            Err(ParseError::Incomplete) => {
                // 需要更多数据
                continue;
            }
            Err(e) => {
                // 解析错误，返回 400
                let response = Response::new(400).body(b"Bad Request".to_vec());
                let response_bytes = response.to_bytes();
                stream.write_all(&response_bytes).await?;
                return Ok(());
            }
        }
    }
}

fn handle_request(request: &crate::http::Request, root: &std::path::Path) -> Response {
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
