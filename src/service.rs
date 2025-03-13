use std::{
    net::SocketAddr,
    pin::pin,
    sync::Arc,
    time::{self, Duration},
};

use crate::{
    config::SettingHost,
    error::Error,
    http::{internal_server_error, not_found, CandyHandler},
    utils::{io_error, load_certs, load_private_key},
};

use futures_util::Future;
use http::Request;
use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::{self, graceful::GracefulShutdown},
};
use rustls::ServerConfig;
use tokio::{
    net::{TcpListener, TcpStream},
    select,
};

use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

impl SettingHost {
    pub fn mk_server(&'static self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        async move {
            let listener = TcpListener::bind(&addr).await?;
            info!("host bind on {}", addr);

            let server = server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
            let graceful = server::graceful::GracefulShutdown::new();
            let mut ctrl_c = pin!(tokio::signal::ctrl_c());

            // load ssl certificate
            let tls_acceptor: Option<TlsAcceptor> =
                if self.certificate.is_some() && self.certificate_key.is_some() {
                    // Set a process wide default crypto provider.
                    #[cfg(feature = "ring")]
                    let _ = rustls::crypto::ring::default_provider().install_default();
                    #[cfg(feature = "aws-lc-rs")]
                    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
                    info!("load ssl certificate");
                    // Load public certificate.
                    let certs = load_certs(self.certificate.as_ref().unwrap())?;
                    info!("load ssl private key");
                    // Load private key.
                    let key = load_private_key(self.certificate_key.as_ref().unwrap())?;
                    // Build TLS configuration.
                    let mut server_config = ServerConfig::builder()
                        .with_no_client_auth()
                        .with_single_cert(certs, key)
                        .map_err(|e| io_error(e.to_string()))?;
                    server_config.alpn_protocols =
                        vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
                    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
                    Some(tls_acceptor)
                } else {
                    None
                };

            loop {
                let tls_acceptor = tls_acceptor.clone();
                tokio::select! {
                    conn = listener.accept() => {
                        let conn = match conn {
                            Ok(conn) => conn,
                            Err(e) => {
                                error!("accept error: {}", e);
                                continue;
                            }
                        };
                        handle_connection(conn, self, &server, &graceful, tls_acceptor).await;
                    },
                    _ = ctrl_c.as_mut() => {
                        drop(listener);
                        info!("Ctrl-C received, starting shutdown");
                        break;
                    }
                }
            }

            select! {
                _ = graceful.shutdown() => {
                    info!("Gracefully shutdown!");
                },
                _ = tokio::time::sleep(Duration::from_secs(self.timeout.into())) => {
                    error!("Waited 10 seconds for graceful shutdown, aborting...");
                }
            }
            Ok(())
        }
    }
}

/// Use to handle connection
///
/// ## Arguments
///
/// `$stream`: TcpStream or TlsStream
/// `$server`: hyper_util server Builder
/// `$service`: hyper service
/// `$graceful`: hyper_util server graceful shutdown
/// `$peer_addr`: SocketAddr
macro_rules! handle_connection {
    ($stream:expr, $server:expr, $service:expr, $graceful:expr, $peer_addr:expr) => {
        let stream = TokioIo::new(Box::pin($stream));
        let conn =
            $server.serve_connection_with_upgrades(stream, hyper::service::service_fn($service));
        let conn = $graceful.watch(conn.into_owned());
        tokio::spawn(async move {
            if let Err(err) = conn.await {
                error!("connection error: {}", err);
            }
            debug!("connection dropped: {}", $peer_addr);
        });
    };
}

/// Handle tcp connection from client
/// then use hyper service to handle response
///
/// ## Arguments
///
/// `conn`: connection accepted from TcpListener
/// `host`: SettingHost from config file
/// `server`: hyper_util server Builder
/// `graceful`: hyper_util server graceful shutdown
async fn handle_connection(
    conn: (TcpStream, SocketAddr),
    host: &'static SettingHost,
    server: &server::conn::auto::Builder<TokioExecutor>,
    graceful: &GracefulShutdown,
    tls_acceptor: Option<TlsAcceptor>,
) {
    let (stream, peer_addr) = conn;
    debug!("incomming connection accepted: {}", peer_addr);

    let service = move |req: Request<Incoming>| async move {
        let start_time = time::Instant::now();
        let method = req.method().clone();
        let uri = req.uri().clone();
        let path = uri.path();
        let version = req.version();
        let mut handler = CandyHandler::new(req, host);
        // Connection handler in service_fn
        // then decide whether to handle proxy or static file based on config
        handler
            .add_headers()
            .map_err(|err| error!("add headers to response failed {}", err))
            .ok();
        let res = handler.handle().await;
        let response = match res {
            Ok(res) => res,
            Err(Error::NotFound(err)) => {
                warn!("{err}");
                not_found()
            }
            Err(err) => {
                error!("{err}");
                internal_server_error()
            }
        };
        let instant_elapsed = start_time.elapsed();
        let micros = instant_elapsed.as_micros();
        let millis = instant_elapsed.as_millis();
        let end_time = if micros >= 1000 {
            format!("{millis:.3}ms")
        } else {
            format!("{micros:.3}μs")
        };
        let res_status = response.status();
        info!("\"{peer_addr}\" {method} {path} {version:?} {res_status} {end_time}");
        anyhow::Ok(response)
    };

    if host.certificate.is_some() && host.certificate_key.is_some() {
        let tls_acceptor = if let Some(tls_acceptor) = tls_acceptor {
            tls_acceptor
        } else {
            warn!("tls_acceptor is None");
            return;
        };
        let tls_stream = match tls_acceptor.accept(stream).await {
            Ok(tls_stream) => tls_stream,
            Err(err) => {
                debug!("failed to perform tls handshake: {err:#}");
                return;
            }
        };
        handle_connection!(tls_stream, server, service, graceful, peer_addr);
    } else {
        handle_connection!(stream, server, service, graceful, peer_addr);
    }
}
