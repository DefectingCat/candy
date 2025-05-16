use std::{
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::anyhow;
use axum::{Router, extract::Request, middleware, routing::get};
use dashmap::DashMap;
use futures_util::pin_mut;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
};
use tower::{Service, ServiceBuilder};
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::{debug, error, info, warn};

use crate::{
    config::SettingHost,
    middlewares::{add_headers, add_version, logging_route},
    utils::{shutdown, shutdown_signal},
};

pub mod error;
pub mod serve;

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

// static ROUTE_MAP: LazyLock<RwLock<HostRouteMap>> = LazyLock::new(|| RwLock::new(BTreeMap::new()));

pub async fn make_server(host: SettingHost) -> anyhow::Result<()> {
    let mut router = Router::new();
    let host_to_save = host.clone();
    // find routes in config
    // convert to axum routes
    // register routes
    for host_route in &host.route {
        // reverse proxy
        if host_route.proxy_pass.is_some() {
            continue;
            // router = router.route(host_route.location.as_ref(), get(hello));
        }

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
            let path_without_slash = host_route.location.chars().collect::<Vec<_>>()[0..len - 1]
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
            // register path  /doc/
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
    let listener = TcpListener::bind(&addr).await?;
    info!("listening on {}", addr);

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
        let rustls_config = rustls_server_config(key, cert)?;
        let tls_acceptor = TlsAcceptor::from(rustls_config);

        pin_mut!(listener);
        loop {
            let tower_service = router.clone();
            let tls_acceptor = tls_acceptor.clone();

            // Wait for new tcp connecttion
            let (cnx, addr) = match listener.accept().await {
                Ok((cnx, addr)) => (cnx, addr),
                Err(err) => {
                    error!("TCP connection accept error: {:?}", err);
                    continue;
                }
            };

            let tls_handler = async move {
                // Wait for tls handshake to happen
                let Ok(stream) = tls_acceptor.accept(cnx).await else {
                    error!("error during tls handshake connection from {}", addr);
                    return;
                };

                // Hyper has its own `AsyncRead` and `AsyncWrite` traits and doesn't use tokio.
                // `TokioIo` converts between them.
                let stream = TokioIo::new(stream);

                // Hyper also has its own `Service` trait and doesn't use tower. We can use
                // `hyper::service::service_fn` to create a hyper `Service` that calls our app through
                // `tower::Service::call`.
                let hyper_service =
                    hyper::service::service_fn(move |request: Request<Incoming>| {
                        // We have to clone `tower_service` because hyper's `Service` uses `&self` whereas
                        // tower's `Service` requires `&mut self`.
                        //
                        // We don't need to call `poll_ready` since `Router` is always ready.
                        tower_service.clone().call(request)
                    });

                let ret = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                    .serve_connection_with_upgrades(stream, hyper_service)
                    .await;

                if let Err(err) = ret {
                    warn!("error serving connection from {}: {}", addr, err);
                }
            };
            tokio::spawn(tls_handler);
        }
    } else {
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal(shutdown))
            .await?;
    }

    Ok(())
}

/// Creates a Rustls `ServerConfig` for TLS-enabled connections.
///
/// # Arguments
/// - `key`: Path to the PEM-encoded private key file.
/// - `cert`: Path to the PEM-encoded certificate chain file.
///
/// # Returns
/// - `Ok(Arc<ServerConfig>)`: A configured `ServerConfig` with:
///   - No client authentication.
///   - ALPN protocols `h2` and `http/1.1` for HTTP/2 and HTTP/1.1 support.
///   - The provided certificate and private key.
/// - `Err(anyhow::Error)`: If the key/cert files are missing, malformed, or invalid.
///
/// # Errors
/// - Fails if:
///   - The private key or certificate files cannot be read or parsed.
///   - The key/cert pair is incompatible (e.g., mismatched algorithms).
///   - The certificate chain is empty or invalid.
///
/// # Example
/// ```rust
/// let config = rustls_server_config("key.pem", "cert.pem")?;
fn rustls_server_config(
    key: impl AsRef<Path>,
    cert: impl AsRef<Path>,
) -> anyhow::Result<Arc<ServerConfig>> {
    let key = PrivateKeyDer::from_pem_file(key)?;

    let certs = CertificateDer::pem_file_iter(cert)?.try_collect()?;

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("bad certificate/key");

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}
