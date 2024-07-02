use std::{
    net::SocketAddr,
    path::Path,
    pin::pin,
    time::{self, Duration, Instant},
};

use crate::{
    config::{SettingHost, SettingRoute},
    consts::{NAME, VERSION},
    error::{Error, Result},
    http::{
        handle_get, handle_not_found, internal_server_error, not_found, CandyBody, CandyHandler,
    },
    utils::{find_route, parse_assets_path},
};

use anyhow::{anyhow, Context};
use futures_util::Future;
use http::{response::Builder, Method, Request, Response};
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::{self, graceful::GracefulShutdown},
};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
};

use tracing::{debug, error, info, instrument, warn};

impl SettingHost {
    pub fn mk_server(&'static self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        async move {
            let listener = TcpListener::bind(&addr).await?;
            info!("host bind on {}", addr);

            let server = server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
            let graceful = server::graceful::GracefulShutdown::new();
            let mut ctrl_c = pin!(tokio::signal::ctrl_c());

            loop {
                tokio::select! {
                    conn = listener.accept() => {
                        let conn = match conn {
                            Ok(conn) => conn,
                            Err(e) => {
                                error!("accept error: {}", e);
                                continue;
                            }
                        };
                        handle_connection(conn, self, &server, &graceful).await;
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
) {
    let (stream, peer_addr) = conn;
    debug!("incomming connection accepted: {}", peer_addr);

    let stream = TokioIo::new(Box::pin(stream));

    let service = move |req: Request<Incoming>| async move {
        let start_time = time::Instant::now();
        let res = handle_service(&req, host).await;
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
        let end_time = (Instant::now() - start_time).as_micros() as f32;
        let end_time = end_time / 1000_f32;
        let method = &req.method();
        let path = &req.uri().path();
        let version = &req.version();
        let res_status = response.status();
        info!(
            "\"{}\" {} {} {:?} {} {:.3}ms",
            peer_addr, method, path, version, res_status, end_time
        );
        anyhow::Ok(response)
    };

    let conn = server.serve_connection_with_upgrades(stream, hyper::service::service_fn(service));
    let conn = graceful.watch(conn.into_owned());

    tokio::spawn(async move {
        if let Err(err) = conn.await {
            error!("connection error: {}", err);
        }
        debug!("connection dropped: {}", peer_addr);
    });
}

/// Connection handler in service_fn
/// then decide whether to handle proxy or static file based on config
pub async fn handle_service(
    req: &Request<Incoming>,
    host: &'static SettingHost,
) -> Result<Response<CandyBody<Bytes>>> {
    let mut handler = CandyHandler::new(req, host)?;
    handler.add_headers()?;
    handler.handle().await
}
