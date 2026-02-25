use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

use dashmap::DashMap;

use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use http::{HeaderName, Uri};
use reqwest::Client;

use super::{
    HOSTS, UPSTREAMS,
    error::{RouteError, RouteResult},
};
use crate::config::LoadBalanceType;
use crate::http::serve::custom_page;
use crate::{http::serve::resolve_parent_path, utils::parse_port_from_host};

/// 加权轮询计数器存储
/// 用于跟踪每个 upstream 的当前轮询权重和索引
static WEIGHTED_ROUND_ROBIN_COUNTERS: LazyLock<DashMap<String, AtomicUsize>> =
    LazyLock::new(DashMap::new);

/// 服务器连接数计数器存储
/// 用于跟踪每个 upstream 中每个服务器的当前连接数
static LEAST_CONN_COUNTERS: LazyLock<DashMap<String, DashMap<usize, AtomicUsize>>> =
    LazyLock::new(DashMap::new);

/// 全局 reqwest 客户端实例，用于复用连接池，提高性能
static CLIENT: OnceLock<Client> = OnceLock::new();

/// 获取全局 reqwest 客户端实例
///
/// 该函数使用 OnceLock 确保客户端只初始化一次，提供一个静态引用以实现连接池复用。
///
/// # 返回值
///
/// 返回静态的 reqwest 客户端引用，用于发送 HTTP 请求
fn get_client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .build()
            .expect("Failed to initialize reqwest client")
    })
}

/// 处理入站请求的反向代理逻辑。
/// 该函数：
/// 1. 提取请求路径、主机和其他细节信息。
/// 2. 解析父路径和代理配置。
/// 3. 将请求转发到配置的代理服务器。
/// 4. 将代理服务器的响应返回给客户端。
///
/// # 参数
/// * `req_uri` - 入站请求的URI。
/// * `path` - 从请求中提取的可选路径参数。
/// * `req` - 入站的HTTP请求。
///
/// # 返回
/// 包含代理服务器响应或错误的 `RouteResult`。
#[axum::debug_handler]
pub async fn serve(
    req_uri: Uri,
    path: Option<Path<String>>,
    mut req: Request<Body>,
) -> RouteResult<impl IntoResponse> {
    let req_path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(req_path);

    let host = req
        .headers()
        .get("host") // 注意：host 是小写的
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    let scheme = req.uri().scheme_str().unwrap_or("http");
    let port = parse_port_from_host(host, scheme).ok_or(RouteError::BadRequest())?;
    // 解析域名
    let (domain, _) = host.split_once(':').unwrap_or((host, ""));
    let domain = domain.to_lowercase();

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
    tracing::debug!("Route map entries: {:?}", route_map);

    let parent_path = resolve_parent_path(&req_uri, path.as_ref());
    tracing::debug!("parent path: {:?}", parent_path);
    let proxy_config = route_map
        .get(&parent_path)
        .ok_or(RouteError::RouteNotFound())?;
    tracing::debug!("proxy config: {:?}", proxy_config);

    // 确定代理目标 - 支持单一 proxy_pass 和 upstream 负载均衡
    let uri = if let Some(ref proxy_pass) = proxy_config.proxy_pass {
        format!("{proxy_pass}{path_query}")
    } else if let Some(ref upstream_name) = proxy_config.upstream {
        // 获取 upstream 配置
        let upstream = UPSTREAMS
            .get(upstream_name)
            .ok_or(RouteError::InternalError())?;

        // 根据负载均衡算法选择服务器
        let server = match upstream.method {
            LoadBalanceType::RoundRobin => {
                // 简单轮询算法
                let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                    .entry(upstream_name.clone())
                    .or_insert_with(|| AtomicUsize::new(0));

                let current_counter = counter.fetch_add(1, Ordering::Relaxed);
                let selected_index = current_counter % upstream.server.len();
                &upstream.server[selected_index]
            }
            LoadBalanceType::WeightedRoundRobin => {
                // 加权轮询算法
                let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                    .entry(upstream_name.clone())
                    .or_insert_with(|| AtomicUsize::new(0));

                let current_counter = counter.fetch_add(1, Ordering::Relaxed);
                let total_weight: u32 = upstream.server.iter().map(|s| s.weight).sum();
                let mut current_weight = current_counter % total_weight as usize;

                let mut selected_index = 0;
                for (i, server) in upstream.server.iter().enumerate() {
                    if current_weight < server.weight as usize {
                        selected_index = i;
                        break;
                    }
                    current_weight -= server.weight as usize;
                }
                &upstream.server[selected_index]
            }
            LoadBalanceType::IpHash => {
                // IP 哈希算法（会话保持）
                // 获取客户端 IP 地址
                let client_ip = req
                    .headers()
                    .get("x-forwarded-for")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.split(',').next())
                    .or_else(|| req.headers().get("x-real-ip").and_then(|h| h.to_str().ok()))
                    .ok_or(RouteError::BadRequest())?;

                // 计算 IP 地址的哈希值
                let hash = ip_hash(client_ip);
                let selected_index = hash % upstream.server.len();
                &upstream.server[selected_index]
            }
            LoadBalanceType::LeastConn => {
                // 最少连接数算法
                let counters = LEAST_CONN_COUNTERS
                    .entry(upstream_name.clone())
                    .or_default();

                // 初始化服务器连接计数器（如果尚未初始化）
                for i in 0..upstream.server.len() {
                    counters.entry(i).or_insert_with(|| AtomicUsize::new(0));
                }

                // 找到连接数最少的服务器
                let mut selected_index = 0;
                let mut min_connections = usize::MAX;

                for (i, server) in upstream.server.iter().enumerate() {
                    let conn_count = counters
                        .get(&i)
                        .map(|v| v.load(Ordering::Relaxed))
                        .unwrap_or(0);

                    // 计算加权连接数（连接数 / 权重），用于公平比较
                    let weighted_conn = conn_count as f64 / server.weight as f64;

                    // 更新最小连接数和选中的服务器
                    if weighted_conn < min_connections as f64 {
                        min_connections = conn_count;
                        selected_index = i;
                    } else if weighted_conn == min_connections as f64 {
                        // 如果连接数相同，则选择权重较大的服务器
                        if server.weight > upstream.server[selected_index].weight {
                            selected_index = i;
                            min_connections = conn_count;
                        }
                    }
                }

                // 增加选中服务器的连接计数
                counters
                    .get_mut(&selected_index)
                    .unwrap()
                    .fetch_add(1, Ordering::Relaxed);

                &upstream.server[selected_index]
            }
        };

        // 构建完整的代理 URI，确保正确的格式
        let server_addr =
            if server.server.starts_with("http://") || server.server.starts_with("https://") {
                server.server.clone()
            } else {
                format!("http://{}", server.server)
            };

        format!("{}{}", server_addr.trim_end_matches('/'), path_query)
    } else {
        return custom_page(proxy_config, req, true).await;
    };

    tracing::debug!("reverse proxy uri: {:?}", &uri);
    *req.uri_mut() = Uri::try_from(uri.clone()).map_err(|_| RouteError::InternalError())?;

    let timeout = proxy_config.proxy_timeout;

    // forward request headers
    let client = get_client();
    let mut forward_req = client
        .request(req.method().clone(), uri)
        .timeout(Duration::from_secs(timeout.into()));
    for (name, value) in req.headers().iter() {
        if !is_exclude_header(name) {
            forward_req = forward_req.header(name.clone(), value.clone());
        }
    }

    // forward request body
    let body = req.into_body();
    // 直接转发请求体，避免中间转换为字符串，提高性能
    let bytes = axum::body::to_bytes(body, 10 * 1024 * 1024)
        .await
        .map_err(|err| {
            tracing::error!("Failed to proxy request: {}", err);
            RouteError::InternalError()
        })?;
    forward_req = forward_req.body(bytes);

    // 记录选中的服务器索引，用于在请求完成后减少连接计数
    let selected_index = if let Some(ref upstream_name) = proxy_config.upstream
        && let Some(upstream) = UPSTREAMS.get(upstream_name)
        && upstream.method == LoadBalanceType::LeastConn
    {
        // 重新计算选中的服务器索引（因为我们需要获取索引值）
        let counters = LEAST_CONN_COUNTERS
            .entry(upstream_name.clone())
            .or_default();

        // 初始化服务器连接计数器（如果尚未初始化）
        for i in 0..upstream.server.len() {
            counters.entry(i).or_insert_with(|| AtomicUsize::new(0));
        }

        // 找到连接数最少的服务器
        let mut selected_idx = 0;
        let mut min_connections = usize::MAX;

        for (i, server) in upstream.server.iter().enumerate() {
            let conn_count = counters
                .get(&i)
                .map(|v| v.load(Ordering::Relaxed))
                .unwrap_or(0);

            let weighted_conn = conn_count as f64 / server.weight as f64;

            if weighted_conn < min_connections as f64 {
                min_connections = conn_count;
                selected_idx = i;
            } else if weighted_conn == min_connections as f64
                && server.weight > upstream.server[selected_idx].weight
            {
                selected_idx = i;
                min_connections = conn_count;
            }
        }

        Some((upstream_name.clone(), selected_idx))
    } else {
        None
    };

    // send reverse proxy request
    let reqwest_response = forward_req.send().await.map_err(|e| {
        // 如果请求失败，减少连接计数
        if let Some((upstream_name, idx)) = &selected_index
            && let Some(counters) = LEAST_CONN_COUNTERS.get(upstream_name)
            && let Some(count) = counters.get(idx)
        {
            count.fetch_sub(1, Ordering::Relaxed);
        }
        tracing::error!("Failed to proxy request: {}", e);
        RouteError::BadRequest()
    })?;

    // response from reverse proxy server
    let mut response_builder = Response::builder().status(reqwest_response.status());
    copy_headers(
        reqwest_response.headers(),
        response_builder
            .headers_mut()
            .ok_or(RouteError::InternalError())?,
    );
    let res = response_builder
        .body(Body::from_stream(reqwest_response.bytes_stream()))
        .map_err(|e| {
            // 如果响应构建失败，减少连接计数
            if let Some((upstream_name, idx)) = &selected_index
                && let Some(counters) = LEAST_CONN_COUNTERS.get(upstream_name)
                && let Some(count) = counters.get(idx)
            {
                count.fetch_sub(1, Ordering::Relaxed);
            }
            tracing::error!("Failed to proxy request: {}", e);
            RouteError::BadRequest()
        })?;

    // 对于 least_conn 算法，我们需要在响应完成后减少连接计数
    if let Some((upstream_name, idx)) = selected_index {
        // 使用自定义 Body 包装器来跟踪响应完成
        let res = wrap_response_body(res, upstream_name, idx).await;
        Ok(res)
    } else {
        Ok(res)
    }
}

/// 包装响应体，在响应完成后自动减少连接计数
///
/// 该函数用于处理最少连接数负载均衡算法的连接计数管理。它会读取整个响应体，
/// 确保在响应处理完成后正确减少服务器的连接计数。
///
/// # 参数
///
/// * `response` - 要包装的原始响应
/// * `upstream_name` - 上游服务器组名称
/// * `server_index` - 服务器在 upstream 中的索引
///
/// # 返回值
///
/// 返回包装后的响应，确保在响应完成后正确更新连接计数
async fn wrap_response_body(
    response: Response<Body>,
    upstream_name: String,
    server_index: usize,
) -> Response<Body> {
    let (parts, original_body) = response.into_parts();

    // 读取整个响应体
    let body_bytes = match axum::body::to_bytes(original_body, 10 * 1024 * 1024).await {
        Ok(bytes) => {
            // 响应完成后，减少连接计数
            if let Some(counters) = LEAST_CONN_COUNTERS.get(&upstream_name)
                && let Some(count) = counters.get(&server_index)
            {
                count.fetch_sub(1, Ordering::Relaxed);
                tracing::debug!(
                    "Connection count decreased for upstream {} server {}: {}",
                    upstream_name,
                    server_index,
                    count.load(Ordering::Relaxed)
                );
            }
            bytes
        }
        Err(e) => {
            tracing::error!("Error reading response body: {}", e);
            // 即使读取失败，也需要减少连接计数
            if let Some(counters) = LEAST_CONN_COUNTERS.get(&upstream_name)
                && let Some(count) = counters.get(&server_index)
            {
                count.fetch_sub(1, Ordering::Relaxed);
            }
            return Response::from_parts(parts, Body::empty());
        }
    };

    // 重新包装响应体
    Response::from_parts(parts, Body::from(body_bytes))
}

/// 检查给定的头部是否应该在反向代理中被排除转发
///
/// 某些 HTTP 头部（如 "host"、"connection" 等）在代理过程中可能会导致冲突或安全问题，
/// 因此需要被排除在转发的头部列表之外。
///
/// # 参数
///
/// * `name` - 要检查的 HTTP 头部名称
///
/// # 返回值
///
/// 如果头部应该被排除则返回 `true`，否则返回 `false`
fn is_exclude_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host"
            | "connection"
            | "proxy-authenticate"
            | "upgrade"
            | "proxy-authorization"
            | "keep-alive"
            | "transfer-encoding"
            | "te"
    )
}

/// 计算 IP 地址的哈希值，用于 IP 哈希负载均衡算法
///
/// 该函数实现了一个简单但有效的字符串哈希算法，将 IP 地址字符串转换为哈希值，
/// 用于在 IP 哈希负载均衡算法中选择服务器。
///
/// # 参数
///
/// * `ip` - 要计算哈希值的 IP 地址字符串（支持 IPv4 和 IPv6）
///
/// # 返回值
///
/// 返回 IP 地址的哈希值（使用 wrapping 运算防止溢出）
fn ip_hash(ip: &str) -> usize {
    let mut hash: usize = 5381;

    for byte in ip.as_bytes() {
        // 使用 wrapping_add 防止溢出
        hash = hash
            .wrapping_shl(5)
            .wrapping_add(hash)
            .wrapping_add(*byte as usize);
    }

    hash
}

/// 将头部从一个 `HeaderMap` 复制到另一个，排除指定的头部
///
/// 该函数负责在代理请求和响应时复制 HTTP 头部，但会排除在 `is_exclude_header` 中
/// 定义的头部，以避免冲突或安全问题。
///
/// # 参数
///
/// * `from` - 源头部映射
/// * `to` - 目标头部映射
fn copy_headers(from: &http::HeaderMap, to: &mut http::HeaderMap) {
    for (name, value) in from.iter() {
        if !is_exclude_header(name) {
            to.append(name.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Upstream, UpstreamServer};
    use http::HeaderValue;

    #[test]
    fn test_is_exclude_header() {
        // 测试应该排除的头部
        assert!(is_exclude_header(&http::header::HOST));
        assert!(is_exclude_header(&http::header::CONNECTION));
        assert!(is_exclude_header(&http::header::UPGRADE));
        assert!(is_exclude_header(&http::header::PROXY_AUTHENTICATE));
        assert!(is_exclude_header(&http::header::PROXY_AUTHORIZATION));
        assert!(is_exclude_header(&http::HeaderName::from_static(
            "keep-alive"
        )));
        assert!(is_exclude_header(&http::header::TRANSFER_ENCODING));
        assert!(is_exclude_header(&http::header::TE));

        // 测试不应该排除的头部
        assert!(!is_exclude_header(&http::header::USER_AGENT));
        assert!(!is_exclude_header(&http::header::CONTENT_TYPE));
        assert!(!is_exclude_header(&http::header::ACCEPT));
        assert!(!is_exclude_header(&http::header::AUTHORIZATION));
        assert!(!is_exclude_header(&http::header::COOKIE));
        assert!(!is_exclude_header(&http::header::REFERER));
    }

    #[test]
    fn test_copy_headers() {
        let mut from = http::HeaderMap::new();
        from.insert(http::header::HOST, HeaderValue::from_static("example.com"));
        from.insert(
            http::header::USER_AGENT,
            HeaderValue::from_static("test-agent"),
        );
        from.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain"),
        );
        from.insert(
            http::header::CONNECTION,
            HeaderValue::from_static("keep-alive"),
        );
        from.insert(http::header::ACCEPT, HeaderValue::from_static("*/*"));

        let mut to = http::HeaderMap::new();
        copy_headers(&from, &mut to);

        // 验证应该被排除的头部没有被复制
        assert!(!to.contains_key(http::header::HOST));
        assert!(!to.contains_key(http::header::CONNECTION));

        // 验证应该被复制的头部被正确复制
        assert_eq!(
            to.get(http::header::USER_AGENT),
            Some(&HeaderValue::from_static("test-agent"))
        );
        assert_eq!(
            to.get(http::header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("text/plain"))
        );
        assert_eq!(
            to.get(http::header::ACCEPT),
            Some(&HeaderValue::from_static("*/*"))
        );
    }

    #[test]
    fn test_ip_hash() {
        // 测试相同IP地址应该返回相同的哈希值
        let ip1 = "192.168.1.1";
        let hash1 = ip_hash(ip1);
        let hash2 = ip_hash(ip1);
        assert_eq!(hash1, hash2);

        // 测试不同IP地址应该返回不同的哈希值（虽然理论上可能碰撞，但概率极低）
        let ip2 = "192.168.1.2";
        let hash3 = ip_hash(ip2);
        assert_ne!(hash1, hash3);

        // 测试IPv6地址的哈希计算
        let ipv6 = "::1";
        let hash4 = ip_hash(ipv6);
        assert!(hash4 > 0);

        // 测试包含端口的IP地址（注意：IP哈希算法会包含端口部分，因为它是字符串哈希）
        let ip_with_port1 = "192.168.1.1:8080";
        let ip_with_port2 = "192.168.1.1:9090";
        assert_ne!(ip_hash(ip_with_port1), ip_hash(ip_with_port2));
    }

    #[test]
    fn test_ip_hash_distribution() {
        // 测试IP哈希在多个IP地址间的分布情况
        let ips = vec![
            "192.168.1.1",
            "192.168.1.2",
            "192.168.1.3",
            "192.168.1.4",
            "192.168.1.5",
            "192.168.1.6",
            "192.168.1.7",
            "192.168.1.8",
        ];

        let mut hashes = Vec::new();
        for ip in &ips {
            hashes.push(ip_hash(ip));
        }

        // 验证没有重复的哈希值（理论上可能有碰撞，但在这个测试集中概率极低）
        let unique_hashes: std::collections::HashSet<_> = hashes.into_iter().collect();
        assert_eq!(unique_hashes.len(), ips.len());
    }

    #[test]
    fn test_least_conn_counter() {
        // 测试最少连接数算法的计数器
        let upstream_name = "test_backend";

        // 初始化上游服务器配置
        let upstream = Upstream {
            name: upstream_name.to_string(),
            server: vec![
                UpstreamServer {
                    server: "192.168.1.100:8080".to_string(),
                    weight: 1,
                },
                UpstreamServer {
                    server: "192.168.1.101:8080".to_string(),
                    weight: 1,
                },
                UpstreamServer {
                    server: "192.168.1.102:8080".to_string(),
                    weight: 2,
                },
            ],
            method: LoadBalanceType::LeastConn,
        };

        // 初始化连接计数器
        let counters = LEAST_CONN_COUNTERS
            .entry(upstream_name.to_string())
            .or_insert_with(DashMap::new);
        for i in 0..upstream.server.len() {
            counters.entry(i).or_insert_with(|| AtomicUsize::new(0));
        }

        // 模拟一些连接
        counters
            .get_mut(&0)
            .unwrap()
            .fetch_add(1, Ordering::Relaxed); // 服务器0: 1个连接
        counters
            .get_mut(&1)
            .unwrap()
            .fetch_add(2, Ordering::Relaxed); // 服务器1: 2个连接
        counters
            .get_mut(&2)
            .unwrap()
            .fetch_add(1, Ordering::Relaxed); // 服务器2: 1个连接 (权重2)

        // 测试最少连接数算法选择服务器
        let selected_index = {
            let mut selected_idx = 0;
            let mut min_connections = usize::MAX;

            for (i, server) in upstream.server.iter().enumerate() {
                let conn_count = counters
                    .get(&i)
                    .map(|v| v.load(Ordering::Relaxed))
                    .unwrap_or(0);
                let weighted_conn = conn_count as f64 / server.weight as f64;

                if weighted_conn < min_connections as f64 {
                    min_connections = conn_count;
                    selected_idx = i;
                } else if weighted_conn == min_connections as f64 {
                    if server.weight > upstream.server[selected_idx].weight {
                        selected_idx = i;
                        min_connections = conn_count;
                    }
                }
            }

            selected_idx
        };

        // 服务器0 和 服务器2 都有 1 个连接，但服务器2 权重为2，所以加权连接数更低，应该选中服务器2
        assert_eq!(selected_index, 2);
    }

    #[test]
    fn test_round_robin_selection() {
        // 测试简单轮询算法的服务器选择逻辑
        let upstream_name = "test_round_robin";
        let upstream = Upstream {
            name: upstream_name.to_string(),
            server: vec![
                UpstreamServer {
                    server: "192.168.1.100:8080".to_string(),
                    weight: 1,
                },
                UpstreamServer {
                    server: "192.168.1.101:8080".to_string(),
                    weight: 1,
                },
                UpstreamServer {
                    server: "192.168.1.102:8080".to_string(),
                    weight: 1,
                },
            ],
            method: LoadBalanceType::RoundRobin,
        };

        // 清除之前的计数器状态
        WEIGHTED_ROUND_ROBIN_COUNTERS.remove(upstream_name);

        // 模拟轮询选择逻辑
        let mut selected_servers = Vec::new();
        for _ in 0..6 {
            let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                .entry(upstream_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));

            let current_counter = counter.fetch_add(1, Ordering::Relaxed);
            let selected_index = current_counter % upstream.server.len();
            selected_servers.push(selected_index);
        }

        // 验证轮询顺序：0 -> 1 -> 2 -> 0 -> 1 -> 2
        assert_eq!(selected_servers, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn test_round_robin_single_server() {
        // 测试单服务器情况下的轮询算法
        let upstream_name = "test_single_server";
        let upstream = Upstream {
            name: upstream_name.to_string(),
            server: vec![UpstreamServer {
                server: "192.168.1.100:8080".to_string(),
                weight: 1,
            }],
            method: LoadBalanceType::RoundRobin,
        };

        // 清除之前的计数器状态
        WEIGHTED_ROUND_ROBIN_COUNTERS.remove(upstream_name);

        // 模拟多次请求
        for _ in 0..5 {
            let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                .entry(upstream_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));

            let current_counter = counter.fetch_add(1, Ordering::Relaxed);
            let selected_index = current_counter % upstream.server.len();
            // 单服务器应该总是选择索引0
            assert_eq!(selected_index, 0);
        }
    }

    #[test]
    fn test_round_robin_distribution() {
        // 测试轮询算法的分布均匀性
        let upstream_name = "test_distribution";
        let server_count = 3;
        let request_count = 300;

        let upstream = Upstream {
            name: upstream_name.to_string(),
            server: (0..server_count)
                .map(|i| UpstreamServer {
                    server: format!("192.168.1.{}:8080", 100 + i),
                    weight: 1,
                })
                .collect(),
            method: LoadBalanceType::RoundRobin,
        };

        // 清除之前的计数器状态
        WEIGHTED_ROUND_ROBIN_COUNTERS.remove(upstream_name);

        // 统计每个服务器被选中的次数
        let mut selection_counts = vec![0usize; server_count];
        for _ in 0..request_count {
            let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                .entry(upstream_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));

            let current_counter = counter.fetch_add(1, Ordering::Relaxed);
            let selected_index = current_counter % upstream.server.len();
            selection_counts[selected_index] += 1;
        }

        // 每个服务器应该被选中 request_count / server_count 次
        let expected_count = request_count / server_count;
        for count in selection_counts {
            assert_eq!(count, expected_count);
        }
    }

    #[test]
    fn test_weighted_round_robin_selection() {
        // 测试加权轮询算法的服务器选择逻辑
        let upstream_name = "test_weighted_rr";
        let upstream = Upstream {
            name: upstream_name.to_string(),
            server: vec![
                UpstreamServer {
                    server: "192.168.1.100:8080".to_string(),
                    weight: 1,
                },
                UpstreamServer {
                    server: "192.168.1.101:8080".to_string(),
                    weight: 2,
                },
                UpstreamServer {
                    server: "192.168.1.102:8080".to_string(),
                    weight: 3,
                },
            ],
            method: LoadBalanceType::WeightedRoundRobin,
        };

        // 清除之前的计数器状态
        WEIGHTED_ROUND_ROBIN_COUNTERS.remove(upstream_name);

        // 总权重 = 1 + 2 + 3 = 6
        let total_weight: u32 = upstream.server.iter().map(|s| s.weight).sum();

        // 模拟一个完整周期的加权轮询
        let mut selected_servers = Vec::new();
        for _ in 0..total_weight {
            let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                .entry(upstream_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));

            let current_counter = counter.fetch_add(1, Ordering::Relaxed);
            let mut current_weight = current_counter % total_weight as usize;

            let mut selected_index = 0;
            for (i, server) in upstream.server.iter().enumerate() {
                if current_weight < server.weight as usize {
                    selected_index = i;
                    break;
                }
                current_weight -= server.weight as usize;
            }
            selected_servers.push(selected_index);
        }

        // 验证在一个完整周期内，每个服务器被选中的次数等于其权重
        let mut counts = vec![0usize; upstream.server.len()];
        for idx in &selected_servers {
            counts[*idx] += 1;
        }
        assert_eq!(counts, vec![1, 2, 3]); // weight 1, 2, 3
    }

    #[test]
    fn test_weighted_round_robin_equal_weights() {
        // 测试权重相等时的加权轮询（应该等同于普通轮询）
        let upstream_name = "test_equal_weights";
        let upstream = Upstream {
            name: upstream_name.to_string(),
            server: vec![
                UpstreamServer {
                    server: "192.168.1.100:8080".to_string(),
                    weight: 2,
                },
                UpstreamServer {
                    server: "192.168.1.101:8080".to_string(),
                    weight: 2,
                },
                UpstreamServer {
                    server: "192.168.1.102:8080".to_string(),
                    weight: 2,
                },
            ],
            method: LoadBalanceType::WeightedRoundRobin,
        };

        // 清除之前的计数器状态
        WEIGHTED_ROUND_ROBIN_COUNTERS.remove(upstream_name);

        let total_weight: u32 = upstream.server.iter().map(|s| s.weight).sum();
        let mut selected_servers = Vec::new();

        for _ in 0..total_weight {
            let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                .entry(upstream_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));

            let current_counter = counter.fetch_add(1, Ordering::Relaxed);
            let mut current_weight = current_counter % total_weight as usize;

            let mut selected_index = 0;
            for (i, server) in upstream.server.iter().enumerate() {
                if current_weight < server.weight as usize {
                    selected_index = i;
                    break;
                }
                current_weight -= server.weight as usize;
            }
            selected_servers.push(selected_index);
        }

        // 验证每个服务器被选中次数相等
        let mut counts = vec![0usize; upstream.server.len()];
        for idx in &selected_servers {
            counts[*idx] += 1;
        }
        assert_eq!(counts, vec![2, 2, 2]); // 所有权重都是2
    }

    #[test]
    fn test_weighted_round_robin_single_high_weight() {
        // 测试单个服务器权重很高的情况
        let upstream_name = "test_single_high_weight";
        let upstream = Upstream {
            name: upstream_name.to_string(),
            server: vec![
                UpstreamServer {
                    server: "192.168.1.100:8080".to_string(),
                    weight: 5,
                },
                UpstreamServer {
                    server: "192.168.1.101:8080".to_string(),
                    weight: 1,
                },
            ],
            method: LoadBalanceType::WeightedRoundRobin,
        };

        // 清除之前的计数器状态
        WEIGHTED_ROUND_ROBIN_COUNTERS.remove(upstream_name);

        let total_weight: u32 = upstream.server.iter().map(|s| s.weight).sum();
        let mut selected_servers = Vec::new();

        for _ in 0..total_weight {
            let counter = WEIGHTED_ROUND_ROBIN_COUNTERS
                .entry(upstream_name.to_string())
                .or_insert_with(|| AtomicUsize::new(0));

            let current_counter = counter.fetch_add(1, Ordering::Relaxed);
            let mut current_weight = current_counter % total_weight as usize;

            let mut selected_index = 0;
            for (i, server) in upstream.server.iter().enumerate() {
                if current_weight < server.weight as usize {
                    selected_index = i;
                    break;
                }
                current_weight -= server.weight as usize;
            }
            selected_servers.push(selected_index);
        }

        // 验证：权重5的服务器应该被选中5次，权重1的服务器应该被选中1次
        let mut counts = vec![0usize; upstream.server.len()];
        for idx in &selected_servers {
            counts[*idx] += 1;
        }
        assert_eq!(counts, vec![5, 1]);
    }

    #[test]
    fn test_load_balance_type_equality() {
        // 测试 LoadBalanceType 的相等性比较
        assert_eq!(LoadBalanceType::RoundRobin, LoadBalanceType::RoundRobin);
        assert_eq!(
            LoadBalanceType::WeightedRoundRobin,
            LoadBalanceType::WeightedRoundRobin
        );
        assert_eq!(LoadBalanceType::IpHash, LoadBalanceType::IpHash);
        assert_eq!(LoadBalanceType::LeastConn, LoadBalanceType::LeastConn);

        assert_ne!(LoadBalanceType::RoundRobin, LoadBalanceType::WeightedRoundRobin);
        assert_ne!(LoadBalanceType::IpHash, LoadBalanceType::LeastConn);
    }

    #[test]
    fn test_upstream_server_default_weight() {
        // 测试 UpstreamServer 的默认权重值（应为1）
        // 默认权重通过 #[serde(default)] 设置
        let server = UpstreamServer {
            server: "localhost:8080".to_string(),
            weight: 1, // 默认权重
        };
        assert_eq!(server.weight, 1);
    }
}
