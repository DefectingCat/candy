use std::{net::SocketAddr, sync::LazyLock, time::Duration};

use anyhow::anyhow;
use axum::{Router, middleware, routing::get};
use axum_server::tls_rustls::RustlsConfig;
use dashmap::DashMap;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::{debug, info, warn};

use crate::{
    config::SettingHost,
    middlewares::{add_headers, add_version, logging_route},
    utils::{shutdown, shutdown_signal},
};

pub mod error;
// handle static file
pub mod serve;
// handle reverse proxy
pub mod reverse_proxy;

/// Host configuration
/// use virtual host port as key
/// use SettingHost as value
/// Use port as parent part
/// Use host.route.location as key
/// Use host.route struct as value
/// {
///     80: {
///         "/doc": <SettingRoute>
///     }
/// }
pub static HOSTS: LazyLock<DashMap<u16, SettingHost>> = LazyLock::new(DashMap::new);

pub async fn make_server(host: SettingHost) -> anyhow::Result<()> {
    let mut router = Router::new();
    let host_to_save = host.clone();
    // find routes in config
    // convert to axum routes
    // register routes
    for host_route in &host.route {
        // reverse proxy
        if host_route.proxy_pass.is_some() {
            router = router.route(host_route.location.as_ref(), get(reverse_proxy::serve));
            // save route path to map
            {
                host_to_save
                    .route_map
                    .insert(host_route.location.clone(), host_route.clone());
            }
        } else {
            // static file
            if host_route.root.is_none() {
                warn!("root field not found for route: {:?}", host_route.location);
                continue;
            }
            // resister with location
            // location = "/doc"
            // route: GET /doc/*
            // resister with file path
            // index = ["index.html", "index.txt"]
            // route: GET /doc/index.html
            // route: GET /doc/index.txt
            // register parent path /doc
            let path_morethan_one = host_route.location.len() > 1;
            let route_path = if path_morethan_one && host_route.location.ends_with('/') {
                // first register path with slash /doc
                router = router.route(&host_route.location, get(serve::serve));
                debug!("registed route {}", host_route.location);
                let len = host_route.location.len();
                let path_without_slash = host_route.location.chars().collect::<Vec<_>>()
                    [0..len - 1]
                    .iter()
                    .collect::<String>();
                // then register path without slash /doc/
                router = router.route(&path_without_slash, get(serve::serve));
                debug!("registed route {}", path_without_slash);
                host_route.location.clone()
            } else if path_morethan_one {
                // first register path without slash /doc
                router = router.route(&host_route.location, get(serve::serve));
                debug!("registed route {}", host_route.location);
                // then register path with slash /doc/
                let path = format!("{}/", host_route.location);
                router = router.route(&path, get(serve::serve));
                debug!("registed route {}", path);
                path
            } else {
                // register path /doc/
                router = router.route(&host_route.location, get(serve::serve));
                debug!("registed route {}", host_route.location);
                host_route.location.clone()
            };
            // save route path to map
            {
                host_to_save
                    .route_map
                    .insert(route_path.clone(), host_route.clone());
            }
            let route_path = format!("{route_path}{{*path}}");
            // register wildcard path /doc/*
            router = router.route(route_path.as_ref(), get(serve::serve));
            debug!("registed route: {}", route_path);
        }
    }

    // save host to map
    HOSTS.insert(host.port, host_to_save);

    router = router.layer(
        ServiceBuilder::new()
            .layer(middleware::from_fn(add_version))
            .layer(middleware::from_fn(add_headers))
            .layer(TimeoutLayer::new(Duration::from_secs(host.timeout.into())))
            .layer(CompressionLayer::new()),
    );

    router = logging_route(router);

    let addr = format!("{}:{}", host.ip, host.port);

    // check ssl eanbled or not
    // if ssl enabled
    // then create ssl listener
    // else create tcp listener
    if host.ssl && host.certificate.is_some() && host.certificate_key.is_some() {
        let cert = host
            .certificate
            .as_ref()
            .ok_or(anyhow!("certificate not found"))?;
        let key = host
            .certificate_key
            .as_ref()
            .ok_or(anyhow!("certificate_key not found"))?;
        debug!("certificate {} certificate_key {}", cert, key);

        let rustls_config = RustlsConfig::from_pem_file(cert, key).await?;
        let addr: SocketAddr = addr.parse()?;
        info!("listening on https://{}", addr);
        axum_server::bind_rustls(addr, rustls_config)
            .serve(router.into_make_service())
            .await?;
    } else {
        let listener = TcpListener::bind(&addr).await?;
        info!("listening on http://{}", addr);
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal(shutdown))
            .await?;
    }

    Ok(())
}
