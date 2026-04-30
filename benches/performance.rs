use candy::http::{Parser, Response};
use candy::compress::{gzip_compress, parse_accept_encoding, should_compress};
use candy::router::{get_mime_type, resolve_path};
use bytes::BytesMut;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};

/// HTTP 解析器基准测试
fn http_parser_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_parser");

    // 简单 GET 请求
    let simple_request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    group.throughput(Throughput::Bytes(simple_request.len() as u64));
    group.bench_function("parse_simple_get", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            let mut buffer = BytesMut::from(&simple_request[..]);
            let _ = parser.parse(&mut buffer);
        });
    });

    // 带多个头部的 GET 请求
    let headers_request = b"GET /index.html HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: Mozilla/5.0\r\nAccept: text/html\r\nAccept-Encoding: gzip\r\nConnection: keep-alive\r\n\r\n";
    group.throughput(Throughput::Bytes(headers_request.len() as u64));
    group.bench_function("parse_with_headers", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            let mut buffer = BytesMut::from(&headers_request[..]);
            let _ = parser.parse(&mut buffer);
        });
    });

    // POST 请求带 body
    let post_request = b"POST /api/submit HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 27\r\n\r\n{\"message\": \"Hello World\"}";
    group.throughput(Throughput::Bytes(post_request.len() as u64));
    group.bench_function("parse_post_with_body", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            let mut buffer = BytesMut::from(&post_request[..]);
            let _ = parser.parse(&mut buffer);
        });
    });

    group.finish();
}

/// HTTP 响应构建基准测试
fn http_response_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_response");

    // 简单 200 响应
    group.bench_function("build_200_response", |b| {
        b.iter(|| {
            let response = Response::ok()
                .header("Content-Type", "text/plain")
                .body(b"Hello World".to_vec());
            response.to_bytes()
        });
    });

    // 带多个头部的响应
    group.bench_function("build_response_with_headers", |b| {
        b.iter(|| {
            let response = Response::ok()
                .header("Content-Type", "text/html")
                .header("Cache-Control", "max-age=3600")
                .header("X-Custom-Header", "value")
                .body(b"<html><body>Hello</body></html>".to_vec());
            response.to_bytes()
        });
    });

    // 大响应体
    let large_body = vec![b'x'; 65536]; // 64KB
    group.throughput(Throughput::Bytes(large_body.len() as u64));
    group.bench_function("build_large_response", |b| {
        b.iter(|| {
            let response = Response::ok()
                .header("Content-Type", "application/octet-stream")
                .body(large_body.clone());
            response.to_bytes()
        });
    });

    group.finish();
}

/// 压缩基准测试
fn compression_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");

    // 小文件压缩
    let small_data = b"Hello World! This is a test string for compression.";
    group.bench_function("gzip_compress_small", |b| {
        b.iter(|| {
            gzip_compress(small_data)
        });
    });

    // 中等文件压缩 (1KB)
    let medium_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    group.throughput(Throughput::Bytes(medium_data.len() as u64));
    group.bench_function("gzip_compress_medium", |b| {
        b.iter(|| {
            gzip_compress(&medium_data)
        });
    });

    // 大文件压缩 (64KB)
    let large_data: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();
    group.throughput(Throughput::Bytes(large_data.len() as u64));
    group.bench_function("gzip_compress_large", |b| {
        b.iter(|| {
            gzip_compress(&large_data)
        });
    });

    // Accept-Encoding 解析
    group.bench_function("parse_accept_encoding", |b| {
        b.iter(|| {
            parse_accept_encoding(b"gzip, deflate, br")
        });
    });

    // MIME 类型压缩判断
    group.bench_function("should_compress_check", |b| {
        b.iter(|| {
            should_compress("text/html")
        });
    });

    group.finish();
}

/// 路径解析基准测试
fn path_resolution_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_resolution");

    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    // 创建测试文件
    std::fs::write(root.join("index.html"), b"<html></html>").unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(root.join("assets/style.css"), b"body {}").unwrap();

    // 解析根路径
    group.bench_function("resolve_root", |b| {
        b.iter(|| {
            resolve_path(root, "/")
        });
    });

    // 解析文件路径
    group.bench_function("resolve_file", |b| {
        b.iter(|| {
            resolve_path(root, "/index.html")
        });
    });

    // 解析嵌套路径
    group.bench_function("resolve_nested_file", |b| {
        b.iter(|| {
            resolve_path(root, "/assets/style.css")
        });
    });

    // 解析不存在的文件
    group.bench_function("resolve_not_found", |b| {
        b.iter(|| {
            resolve_path(root, "/nonexistent.html")
        });
    });

    // MIME 类型查找
    group.bench_function("get_mime_type_html", |b| {
        b.iter(|| {
            get_mime_type(std::path::Path::new("index.html"))
        });
    });

    group.bench_function("get_mime_type_css", |b| {
        b.iter(|| {
            get_mime_type(std::path::Path::new("style.css"))
        });
    });

    group.bench_function("get_mime_type_js", |b| {
        b.iter(|| {
            get_mime_type(std::path::Path::new("app.js"))
        });
    });

    group.finish();
}

/// Range 请求解析基准测试
fn range_request_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_request");

    // 解析 Range 头
    group.bench_function("parse_range_start_end", |b| {
        b.iter(|| {
            candy::worker::parse_range_header(b"bytes=0-999", 10000)
        });
    });

    group.bench_function("parse_range_start_only", |b| {
        b.iter(|| {
            candy::worker::parse_range_header(b"bytes=500-", 10000)
        });
    });

    group.bench_function("parse_range_suffix", |b| {
        b.iter(|| {
            candy::worker::parse_range_header(b"bytes=-500", 10000)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    http_parser_benchmark,
    http_response_benchmark,
    compression_benchmark,
    path_resolution_benchmark,
    range_request_benchmark,
);

criterion_main!(benches);
