use bytes::{Buf, BytesMut};
use std::fs::File;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, timeout};
use tokio_rustls::TlsAcceptor;

use crate::compress::{
    CompressionType, deflate_compress, gzip_compress, parse_accept_encoding, should_compress,
};
use crate::config::{Config, LogFormat};
use crate::http::{HttpVersion, Method, ParseError, Parser, Request, Response};
use crate::http2::{
    CONNECTION_PREFACE, Connection, FrameType, H2Error, H2ErrorCode, HpackDecoder, HpackEncoder,
    build_data_frame, build_goaway, build_headers_frame, build_initial_settings, build_rst_stream,
    build_settings_ack, parse_frame_header, parse_settings, parse_window_update,
};
use crate::logging::{AccessLog, Logger};
use crate::router::{ResolveResult, get_mime_type, resolve_path};
use crate::sendfile::sendfile_async;
use crate::socket::create_reuseport_listener;
use crate::tls::load_tls_config_with_alpn;

/// Worker 进程主函数
pub fn run(config: &Config) -> std::io::Result<()> {
    println!(
        "Worker {} starting on {}",
        std::process::id(),
        config.server.https_listen
    );

    // 创建 tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let root = config.server.root.clone();
    let keep_alive_timeout = config.http.keep_alive_timeout;
    let max_header_size = config.http.max_header_size;
    let max_body_size = config.http.max_body_size;

    // 加载 TLS 配置（如果启用）
    let tls_acceptor = if let Some(tls_config) = &config.tls {
        if tls_config.enabled {
            match load_tls_config_with_alpn(tls_config) {
                Ok(server_config) => Some(TlsAcceptor::from(server_config)),
                Err(e) => {
                    eprintln!("Failed to load TLS config: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    runtime.block_on(async {
        // 创建 HTTPS listener
        let https_listener = create_reuseport_listener(config.server.https_listen)?;

        println!(
            "Worker {} listening on HTTPS {}",
            std::process::id(),
            config.server.https_listen
        );

        // 如果配置了 HTTP 端口，创建 HTTP listener 用于重定向
        let http_listener = if let Some(http_addr) = config.server.http_listen {
            let listener = create_reuseport_listener(http_addr)?;
            println!(
                "Worker {} listening on HTTP {} (redirect to HTTPS)",
                std::process::id(),
                http_addr
            );
            Some(listener)
        } else {
            None
        };

        // 日志配置
        let log_enabled = config.log.access;
        let log_format = config.log.format;

        // HTTP/2 配置
        let http2_push = config.http.http2_push;

        // 主循环
        loop {
            // 同时接受 HTTP 和 HTTPS 连接
            let https_accept = https_listener.accept();
            let http_accept = async {
                match &http_listener {
                    Some(listener) => listener.accept().await,
                    None => std::future::pending().await,
                }
            };

            tokio::select! {
                // HTTPS 连接
                result = https_accept => {
                    match result {
                        Ok((stream, addr)) => {
                            let root = root.clone();
                            let tls_acceptor = tls_acceptor.clone();
                            tokio::spawn(async move {
                                let start = Instant::now();
                                let result = handle_https_connection(
                                    stream, addr, &root, keep_alive_timeout, max_header_size, max_body_size, tls_acceptor.clone(), http2_push
                                ).await;
                                let duration = start.elapsed().as_millis() as u64;

                                // 记录访问日志
                                if log_enabled {
                                    log_request(
                                        addr.to_string(),
                                        "HTTPS",
                                        duration,
                                        result.is_ok(),
                                        log_format,
                                        None,
                                    );
                                }

                                if let Err(e) = result {
                                    eprintln!("HTTPS connection error from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => eprintln!("HTTPS accept error: {}", e),
                    }
                }
                // HTTP 连接（重定向到 HTTPS）
                result = http_accept => {
                    match result {
                        Ok((stream, addr)) => {
                            let https_addr = config.server.https_listen;
                            tokio::spawn(async move {
                                let start = Instant::now();
                                let result = handle_http_redirect(stream, addr, https_addr, max_header_size, max_body_size).await;
                                let duration = start.elapsed().as_millis() as u64;

                                // 记录访问日志
                                if log_enabled {
                                    log_request(
                                        addr.to_string(),
                                        "HTTP",
                                        duration,
                                        result.is_ok(),
                                        log_format,
                                        None,
                                    );
                                }

                                if let Err(e) = result {
                                    eprintln!("HTTP redirect error from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => eprintln!("HTTP accept error: {}", e),
                    }
                }
            }
        }
    })
}

/// 处理 HTTPS 连接
#[allow(clippy::too_many_arguments)]
async fn handle_https_connection(
    stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    root: &std::path::Path,
    keep_alive_timeout: u64,
    max_header_size: usize,
    max_body_size: usize,
    tls_acceptor: Option<TlsAcceptor>,
    http2_push: bool,
) -> std::io::Result<()> {
    // 如果启用了 TLS，进行 TLS 握手
    if let Some(acceptor) = tls_acceptor {
        let tls_stream = match acceptor.accept(stream).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("TLS handshake error from {}: {}", addr, e);
                return Err(std::io::Error::other(e));
            }
        };

        // 检查 ALPN 协商结果
        let protocol = tls_stream
            .get_ref()
            .1
            .alpn_protocol()
            .map(|p| std::str::from_utf8(p).unwrap_or("unknown"))
            .unwrap_or("http/1.1");

        println!("TLS connection from {} with protocol: {}", addr, protocol);

        // 根据协议选择处理器
        if protocol == "h2" {
            handle_http2_connection_tls(tls_stream, addr, root, keep_alive_timeout, http2_push)
                .await
        } else {
            handle_connection_tls(
                tls_stream,
                addr,
                root,
                keep_alive_timeout,
                max_header_size,
                max_body_size,
            )
            .await
        }
    } else {
        // 无 TLS，直接处理 HTTP
        handle_connection(
            stream,
            addr,
            root,
            keep_alive_timeout,
            max_header_size,
            max_body_size,
        )
        .await
    }
}

/// 处理 TLS 连接（使用 TlsStream）
async fn handle_connection_tls<S>(
    mut stream: S,
    _addr: std::net::SocketAddr,
    root: &std::path::Path,
    keep_alive_timeout: u64,
    max_header_size: usize,
    max_body_size: usize,
) -> std::io::Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buffer = BytesMut::with_capacity(8192);
    let mut parser = Parser::new();
    let mut keep_alive = true;

    while keep_alive {
        // 设置超时读取
        let read_result = timeout(
            Duration::from_secs(keep_alive_timeout),
            read_request_tls(
                &mut stream,
                &mut buffer,
                &mut parser,
                max_header_size,
                max_body_size,
            ),
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

/// TLS 连接读取请求
async fn read_request_tls<S>(
    stream: &mut S,
    buffer: &mut BytesMut,
    parser: &mut Parser,
    max_header_size: usize,
    max_body_size: usize,
) -> Result<Request, ParseError>
where
    S: AsyncReadExt + Unpin,
{
    loop {
        // 尝试解析已有数据
        match parser.parse(buffer, max_header_size, max_body_size) {
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

/// 处理 HTTP/2 连接（TLS）
async fn handle_http2_connection_tls<S>(
    mut stream: S,
    _addr: std::net::SocketAddr,
    root: &std::path::Path,
    keep_alive_timeout: u64,
    http2_push: bool,
) -> std::io::Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buffer = BytesMut::with_capacity(65536);
    let mut connection = Connection::new();
    let mut hpack_decoder = HpackDecoder::new(4096);
    let mut hpack_encoder = HpackEncoder::new(4096);

    // 读取连接前奏（24 字节）
    let preface_read = timeout(
        Duration::from_secs(keep_alive_timeout),
        read_exact(&mut stream, &mut buffer, 24),
    )
    .await;

    match preface_read {
        Ok(Ok(())) => {}
        _ => return Ok(()),
    }

    // 验证连接前奏
    if &buffer[..24] != CONNECTION_PREFACE {
        eprintln!("Invalid HTTP/2 connection preface");
        return Ok(());
    }
    buffer.advance(24);

    // 发送初始 SETTINGS 帧
    let settings_frame = build_initial_settings();
    stream.write_all(&settings_frame).await?;

    // 主循环：处理帧
    loop {
        // 读取帧头（9 字节）
        let frame_read = timeout(
            Duration::from_secs(keep_alive_timeout),
            read_exact(&mut stream, &mut buffer, 9),
        )
        .await;

        match frame_read {
            Ok(Ok(())) => {}
            Ok(Err(_)) | Err(_) => return Ok(()),
        }

        // 解析帧头
        let header = match parse_frame_header(&buffer[..9]) {
            Ok(h) => h,
            Err(_) => {
                let goaway = build_goaway(0, H2ErrorCode::ProtocolError as u32);
                let _ = stream.write_all(&goaway).await;
                return Ok(());
            }
        };
        buffer.advance(9);

        // 读取帧体
        if header.length > 0 {
            let body_read = timeout(
                Duration::from_secs(keep_alive_timeout),
                read_exact(&mut stream, &mut buffer, header.length as usize),
            )
            .await;

            match body_read {
                Ok(Ok(())) => {}
                Ok(Err(_)) | Err(_) => return Ok(()),
            }
        }

        // 处理帧
        let frame_body = buffer.split_to(header.length as usize);
        match header.frame_type {
            FrameType::Settings => {
                if header.flags & 0x01 != 0 {
                    // SETTINGS ACK，忽略
                } else {
                    // 解析并存储客户端设置
                    if let Ok(settings) = parse_settings(&frame_body) {
                        connection.update_peer_settings(settings);
                    }
                    // 发送 SETTINGS ACK
                    let ack = build_settings_ack();
                    stream.write_all(&ack).await?;
                }
            }
            FrameType::Headers => {
                // 处理 HEADERS 帧
                let mut ctx = H2Context {
                    connection: &mut connection,
                    hpack_decoder: &mut hpack_decoder,
                    hpack_encoder: &mut hpack_encoder,
                    root,
                    http2_push,
                };
                if let Err(e) = handle_h2_headers(
                    &mut stream,
                    header.stream_id,
                    &frame_body,
                    header.flags,
                    &mut ctx,
                )
                .await
                {
                    eprintln!("HTTP/2 headers error: {:?}", e);
                    let rst = build_rst_stream(header.stream_id, H2ErrorCode::InternalError as u32);
                    let _ = stream.write_all(&rst).await;
                }
            }
            FrameType::Data => {
                // DATA 帧暂时忽略（我们不处理请求体）
            }
            FrameType::Ping => {
                // PING 帧：回送相同数据
                let mut ping_response = vec![0x00, 0x00, 0x08, 0x06, 0x01, 0x00, 0x00, 0x00, 0x00];
                ping_response.extend_from_slice(&frame_body);
                stream.write_all(&ping_response).await?;
            }
            FrameType::WindowUpdate => {
                // WINDOW_UPDATE：更新流控窗口
                match parse_window_update(&frame_body) {
                    Ok(increment) => {
                        if header.stream_id == 0 {
                            // 连接级窗口更新
                            if let Err(e) = connection.update_connection_send_window(increment) {
                                eprintln!("HTTP/2 connection window update error: {:?}", e);
                                let goaway = build_goaway(0, H2ErrorCode::FlowControlError as u32);
                                let _ = stream.write_all(&goaway).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "flow control error",
                                ));
                            }
                        } else {
                            // 流级窗口更新
                            if let Err(e) =
                                connection.update_stream_send_window(header.stream_id, increment)
                            {
                                eprintln!(
                                    "HTTP/2 stream {} window update error: {:?}",
                                    header.stream_id, e
                                );
                                let rst = build_rst_stream(
                                    header.stream_id,
                                    H2ErrorCode::FlowControlError as u32,
                                );
                                let _ = stream.write_all(&rst).await;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("HTTP/2 WINDOW_UPDATE parse error: {:?}", e);
                    }
                }
            }
            FrameType::GoAway => {
                // GOAWAY：关闭连接
                return Ok(());
            }
            FrameType::RstStream => {
                // RST_STREAM：关闭流
                connection.close_stream(header.stream_id);
            }
            _ => {
                // 忽略其他帧类型
            }
        }
    }
}

/// HTTP/2 请求处理上下文
pub struct H2Context<'a> {
    pub connection: &'a mut Connection,
    pub hpack_decoder: &'a mut HpackDecoder,
    pub hpack_encoder: &'a mut HpackEncoder,
    pub root: &'a std::path::Path,
    pub http2_push: bool,
}

/// 处理 HTTP/2 HEADERS 帧
async fn handle_h2_headers<S>(
    stream: &mut S,
    stream_id: u32,
    frame_body: &[u8],
    _flags: u8,
    ctx: &mut H2Context<'_>,
) -> Result<(), H2Error>
where
    S: AsyncWriteExt + Unpin,
{
    // 接受客户端流
    let _stream = ctx
        .connection
        .accept_client_stream(stream_id)
        .ok_or(H2Error::InvalidStreamId)?;

    // 解码 HPACK 头部
    let headers = ctx.hpack_decoder.decode_headers(frame_body)?;

    // 提取伪头部
    let method = headers
        .get(":method")
        .cloned()
        .unwrap_or_else(|| "GET".to_string());
    let path = headers
        .get(":path")
        .cloned()
        .unwrap_or_else(|| "/".to_string());

    // 构建 HTTP/1.1 风格的请求用于复用现有处理逻辑
    let request = Request {
        method: if method == "GET" {
            Method::GET
        } else if method == "HEAD" {
            Method::HEAD
        } else {
            Method::POST
        },
        path: bytes::Bytes::from(path),
        version: crate::http::HttpVersion::Http11,
        headers: headers
            .iter()
            .filter(|(k, _)| !k.starts_with(':'))
            .map(|(k, v)| (bytes::Bytes::from(k.clone()), bytes::Bytes::from(v.clone())))
            .collect(),
        body: None,
    };

    // 处理请求
    let response = handle_request(&request, ctx.root);

    // 构建响应头
    let status = response.status_code();
    let response_headers: Vec<(String, String)> = vec![
        (":status".to_string(), status.to_string()),
        (
            "content-type".to_string(),
            response.content_type().to_string(),
        ),
        (
            "content-length".to_string(),
            response.body_len().to_string(),
        ),
    ];

    // 编码响应头
    let encoded_headers = ctx.hpack_encoder.encode_headers(&response_headers);

    // 发送 HEADERS 帧
    let body = response.into_body();
    let end_stream = body.is_empty();
    let headers_frame = build_headers_frame(stream_id, encoded_headers, end_stream);
    stream
        .write_all(&headers_frame)
        .await
        .map_err(|_| H2Error::Incomplete)?;

    // 发送 DATA 帧（如果有 body）
    if !body.is_empty() {
        let data_frame = build_data_frame(stream_id, &body, true);
        stream
            .write_all(&data_frame)
            .await
            .map_err(|_| H2Error::Incomplete)?;

        // Server Push: 如果启用且是 HTML，推送关联的 CSS/JS 资源
        if ctx.http2_push
            && response_headers
                .iter()
                .any(|(k, v)| k == "content-type" && v.contains("text/html"))
        {
            let push_resources = extract_push_resources(&body);
            for push_path in push_resources.iter().take(5) {
                // 创建推送流（偶数 ID）
                let push_stream_id = ctx.connection.create_server_stream();

                // 构建推送请求头
                let push_headers: Vec<(String, String)> = vec![
                    (":method".to_string(), "GET".to_string()),
                    (":path".to_string(), push_path.clone()),
                    (":scheme".to_string(), "https".to_string()),
                    (":authority".to_string(), "localhost".to_string()),
                ];
                let encoded_push_headers = ctx.hpack_encoder.encode_headers(&push_headers);

                // 发送 PUSH_PROMISE 帧
                use crate::http2::build_push_promise;
                let push_promise =
                    build_push_promise(stream_id, push_stream_id, encoded_push_headers);
                stream
                    .write_all(&push_promise)
                    .await
                    .map_err(|_| H2Error::Incomplete)?;

                // 读取推送资源并发送
                let push_file_path = ctx.root.join(push_path.trim_start_matches('/'));
                if let Ok(push_content) = std::fs::read(&push_file_path) {
                    let push_headers_response: Vec<(String, String)> = vec![
                        (":status".to_string(), "200".to_string()),
                        (
                            "content-type".to_string(),
                            get_mime_type(&push_file_path).to_string(),
                        ),
                        ("content-length".to_string(), push_content.len().to_string()),
                    ];
                    let encoded_response = ctx.hpack_encoder.encode_headers(&push_headers_response);
                    let push_headers_frame = build_headers_frame(
                        push_stream_id,
                        encoded_response,
                        push_content.is_empty(),
                    );
                    stream
                        .write_all(&push_headers_frame)
                        .await
                        .map_err(|_| H2Error::Incomplete)?;

                    if !push_content.is_empty() {
                        let push_data_frame = build_data_frame(push_stream_id, &push_content, true);
                        stream
                            .write_all(&push_data_frame)
                            .await
                            .map_err(|_| H2Error::Incomplete)?;
                    }
                }

                // 关闭推送流
                ctx.connection.close_stream(push_stream_id);
            }
        }
    }

    // 关闭流
    ctx.connection.close_stream(stream_id);

    Ok(())
}

/// 从 HTML 内容中提取可推送的资源路径
fn extract_push_resources(html: &[u8]) -> Vec<String> {
    let html_str = String::from_utf8_lossy(html);
    let mut resources = Vec::new();

    // 提取 <link rel="stylesheet" href="...">
    for line in html_str.lines() {
        if (line.contains("rel=\"stylesheet\"") || line.contains("rel='stylesheet'"))
            && let Some(href) = extract_attr_value(line, "href")
            && href.starts_with('/')
            && !href.contains("..")
        {
            resources.push(href);
        }
        // 提取 <script src="...">
        if line.contains("<script")
            && line.contains("src=")
            && let Some(src) = extract_attr_value(line, "src")
            && src.starts_with('/')
            && !src.contains("..")
            && src.ends_with(".js")
        {
            resources.push(src);
        }
    }

    resources
}

/// 从 HTML 标签中提取属性值
fn extract_attr_value(tag: &str, attr: &str) -> Option<String> {
    let patterns = [format!("{}=\"", attr), format!("{}='", attr)];
    for pattern in patterns {
        if let Some(start) = tag.find(&pattern) {
            let value_start = start + pattern.len();
            let end_char = if pattern.ends_with("\"") { '"' } else { '\'' };
            if let Some(end) = tag[value_start..].find(end_char) {
                return Some(tag[value_start..value_start + end].to_string());
            }
        }
    }
    None
}

/// 精确读取指定字节数
async fn read_exact<S>(stream: &mut S, buffer: &mut BytesMut, n: usize) -> std::io::Result<()>
where
    S: AsyncReadExt + Unpin,
{
    while buffer.len() < n {
        let mut read_buf = [0u8; 8192];
        let to_read = std::cmp::min(read_buf.len(), n - buffer.len());
        let bytes_read = stream.read(&mut read_buf[..to_read]).await?;

        if bytes_read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected EOF",
            ));
        }

        buffer.extend_from_slice(&read_buf[..bytes_read]);
    }

    Ok(())
}

/// 处理 HTTP 连接，重定向到 HTTPS
async fn handle_http_redirect(
    mut stream: tokio::net::TcpStream,
    _addr: std::net::SocketAddr,
    https_addr: std::net::SocketAddr,
    max_header_size: usize,
    max_body_size: usize,
) -> std::io::Result<()> {
    let mut buffer = BytesMut::with_capacity(4096);
    let mut parser = Parser::new();

    // 读取请求
    let request = loop {
        match parser.parse(&buffer, max_header_size, max_body_size) {
            Ok((_, req)) => break Some(req),
            Err(ParseError::Incomplete) => {
                let mut read_buf = [0u8; 4096];
                let n = stream.read(&mut read_buf).await?;
                if n == 0 {
                    break None;
                }
                buffer.extend_from_slice(&read_buf[..n]);
            }
            Err(_) => break None,
        }
    };

    if let Some(request) = request {
        // 获取 Host 头
        let host = find_header(&request.headers, b"Host")
            .and_then(|h| String::from_utf8(h).ok())
            .unwrap_or_else(|| https_addr.to_string());

        // 构造 HTTPS URL
        let path = String::from_utf8_lossy(&request.path);
        let https_url = if https_addr.port() == 443 {
            format!("https://{}{}", host, path)
        } else {
            format!("https://{}:{}{}", host, https_addr.port(), path)
        };

        // 发送 301 重定向
        let response = Response::new(301)
            .header("Location", https_url)
            .header("Connection", "close")
            .body(b"Moved Permanently".to_vec());

        stream.write_all(&response.to_bytes()).await?;
    }

    Ok(())
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    _addr: std::net::SocketAddr,
    root: &std::path::Path,
    keep_alive_timeout: u64,
    max_header_size: usize,
    max_body_size: usize,
) -> std::io::Result<()> {
    let mut buffer = BytesMut::with_capacity(8192);
    let mut parser = Parser::new();
    let mut keep_alive = true;

    while keep_alive {
        // 设置超时读取
        let read_result = timeout(
            Duration::from_secs(keep_alive_timeout),
            read_request(
                &mut stream,
                &mut buffer,
                &mut parser,
                max_header_size,
                max_body_size,
            ),
        )
        .await;

        match read_result {
            Ok(Ok(request)) => {
                // 检查是否保持连接
                keep_alive = is_keep_alive(&request);

                // 处理请求（尝试使用 sendfile）
                handle_request_with_sendfile(&mut stream, &request, root, keep_alive).await?;
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

/// 处理请求，尽可能使用 sendfile
async fn handle_request_with_sendfile(
    stream: &mut tokio::net::TcpStream,
    request: &Request,
    root: &std::path::Path,
    keep_alive: bool,
) -> std::io::Result<()> {
    // 只支持 GET 和 HEAD
    if request.method != Method::GET && request.method != Method::HEAD {
        let response = Response::new(405)
            .header("Content-Type", "text/plain")
            .header(
                "Connection",
                if keep_alive { "keep-alive" } else { "close" },
            )
            .body(b"Method Not Allowed".to_vec());
        stream.write_all(&response.to_bytes()).await?;
        return Ok(());
    }

    // 解析路径
    let path_str = String::from_utf8_lossy(&request.path);

    match resolve_path(root, &path_str) {
        ResolveResult::File(file_path) => {
            // 尝试打开文件
            let file = File::open(&file_path);
            if file.is_err() {
                let response = Response::internal_error();
                stream.write_all(&response.to_bytes()).await?;
                return Ok(());
            }
            let file = file.unwrap();
            let file_size = file.metadata()?.len() as usize;
            let mime_type = get_mime_type(&file_path);

            // 检查是否可以使用 sendfile
            // 条件：无压缩、无 Range、非 HEAD 请求
            let can_sendfile = !should_compress(mime_type)
                && find_header(&request.headers, b"Range").is_none()
                && request.method == Method::GET
                && find_header(&request.headers, b"Accept-Encoding").is_none();

            if can_sendfile {
                // 发送响应头
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: {}\r\n\r\n",
                    mime_type,
                    file_size,
                    if keep_alive { "keep-alive" } else { "close" }
                );
                stream.write_all(headers.as_bytes()).await?;

                // 使用 sendfile 发送文件内容
                let result = sendfile_async(stream, &file, 0, file_size).await;
                if let Err(e) = result {
                    eprintln!("sendfile error: {}", e);
                    // sendfile 失败，回退到普通读取
                    let content = std::fs::read(&file_path)?;
                    stream.write_all(&content).await?;
                }
            } else {
                // 不能使用 sendfile，使用普通方式
                let content = std::fs::read(&file_path)?;

                // 检查 Range 头
                if let Some(range_header) = find_header(&request.headers, b"Range") {
                    let response = handle_range_request(
                        &range_header,
                        &content,
                        mime_type,
                        file_size,
                        request.method,
                    );
                    let mut response_bytes = response.to_bytes();
                    if keep_alive {
                        response_bytes.extend_from_slice(b"Connection: keep-alive\r\n");
                    }
                    stream.write_all(&response_bytes).await?;
                    return Ok(());
                }

                // 检查压缩协商
                let (final_content, content_encoding) = if should_compress(mime_type) {
                    if let Some(accept_encoding) = find_header(&request.headers, b"Accept-Encoding")
                    {
                        let compression = parse_accept_encoding(&accept_encoding);
                        match compression {
                            CompressionType::Gzip => {
                                if let Ok(compressed) = gzip_compress(&content) {
                                    (compressed, Some("gzip"))
                                } else {
                                    (content, None)
                                }
                            }
                            CompressionType::Deflate => {
                                if let Ok(compressed) = deflate_compress(&content) {
                                    (compressed, Some("deflate"))
                                } else {
                                    (content, None)
                                }
                            }
                            CompressionType::None => (content, None),
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

                if let Some(encoding) = content_encoding {
                    response = response.header("Content-Encoding", encoding);
                }

                if keep_alive {
                    response = response.header("Connection", "keep-alive");
                } else {
                    response = response.header("Connection", "close");
                }

                if request.method == Method::HEAD {
                    response = response.header("Content-Length", final_content.len().to_string());
                } else {
                    response = response.body(final_content);
                }

                stream.write_all(&response.to_bytes()).await?;
            }
        }
        ResolveResult::Directory(_) => {
            let response = Response::forbidden();
            stream.write_all(&response.to_bytes()).await?;
        }
        ResolveResult::NotFound => {
            let response = Response::not_found();
            stream.write_all(&response.to_bytes()).await?;
        }
        ResolveResult::Forbidden => {
            let response = Response::forbidden();
            stream.write_all(&response.to_bytes()).await?;
        }
    }

    Ok(())
}

/// 读取并解析请求
async fn read_request(
    stream: &mut tokio::net::TcpStream,
    buffer: &mut BytesMut,
    parser: &mut Parser,
    max_header_size: usize,
    max_body_size: usize,
) -> Result<Request, ParseError> {
    loop {
        // 尝试解析已有数据
        match parser.parse(buffer, max_header_size, max_body_size) {
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
                            mime_type,
                            file_size,
                            request.method,
                        );
                    }

                    // 检查压缩协商
                    let (final_content, content_encoding) = if should_compress(mime_type) {
                        if let Some(accept_encoding) =
                            find_header(&request.headers, b"Accept-Encoding")
                        {
                            let compression = parse_accept_encoding(&accept_encoding);
                            match compression {
                                CompressionType::Gzip => {
                                    if let Ok(compressed) = gzip_compress(&content) {
                                        (compressed, Some("gzip"))
                                    } else {
                                        (content, None)
                                    }
                                }
                                CompressionType::Deflate => {
                                    if let Ok(compressed) = deflate_compress(&content) {
                                        (compressed, Some("deflate"))
                                    } else {
                                        (content, None)
                                    }
                                }
                                CompressionType::None => (content, None),
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
                        response =
                            response.header("Content-Length", final_content.len().to_string());
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

/// 记录访问日志
fn log_request(
    client_addr: String,
    protocol: &str,
    duration_ms: u64,
    success: bool,
    format: LogFormat,
    request: Option<&Request>,
) {
    let status = if success { 200 } else { 500 };
    let (method, path, version, referer, user_agent, body_size) = request
        .map(|r| {
            let method = String::from_utf8_lossy(match r.method {
                Method::GET => b"GET",
                Method::HEAD => b"HEAD",
                Method::POST => b"POST",
                Method::PUT => b"PUT",
                Method::DELETE => b"DELETE",
                Method::OPTIONS => b"OPTIONS",
                Method::CONNECT => b"CONNECT",
                Method::TRACE => b"TRACE",
                Method::PATCH => b"PATCH",
            })
            .to_string();
            let path = String::from_utf8_lossy(&r.path).to_string();
            let version = match r.version {
                HttpVersion::Http10 => "HTTP/1.0",
                HttpVersion::Http11 => "HTTP/1.1",
            };
            let referer = find_header(&r.headers, b"Referer")
                .map(|v| String::from_utf8_lossy(&v).to_string());
            let user_agent = find_header(&r.headers, b"User-Agent")
                .map(|v| String::from_utf8_lossy(&v).to_string());
            let body_size = r.body.as_ref().map(|b| b.len()).unwrap_or(0);
            (method, path, version, referer, user_agent, body_size)
        })
        .unwrap_or_else(|| {
            (
                protocol.to_string(),
                "/".to_string(),
                "HTTP/1.1",
                None,
                None,
                0,
            )
        });

    // 如果有请求体，记录到日志
    if body_size > 0 {
        println!("Request body size: {} bytes", body_size);
    }

    let log = AccessLog::new(
        client_addr,
        method,
        path,
        version.to_string(),
        status,
        0,
        duration_ms,
    )
    .with_referer(referer)
    .with_user_agent(user_agent);

    let mut logger = Logger::stdout(format);
    if let Err(e) = logger.log_access(&log) {
        eprintln!("Failed to write access log: {}", e);
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
