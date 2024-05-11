use std::{
    error::Error as StdError,
    io::ErrorKind::{NotConnected, NotFound},
    net::SocketAddr,
    path::Path,
    pin::pin,
    time::{self, Duration, Instant},
};

use crate::{
    config::SettingHost,
    error::{Error, Result},
    http::{handle_get, internal_server_error, not_found, CandyBody},
    utils::{find_route, parse_assets_path},
};

use futures_util::Future;
use http::{Method, Request, Response};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use tokio::{
    net::{TcpListener, TcpStream},
    select,
};

use tracing::{debug, error, info, warn};

impl SettingHost {
    pub fn mk_server(&'static self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        #[allow(unreachable_code)]
        async move {
            let listener = TcpListener::bind(&addr).await?;
            info!("host bind on {}", addr);
            loop {
                let socket = listener.accept().await?;

                tokio::spawn(async move {
                    graceful_shutdown(self, socket).await;
                });
            }
            anyhow::Ok(())
        }
    }
}

/// Handle tokio TcpListener socket,
/// then use hyper and service_fn to handle connection
///
/// ## Arguments
///
/// `host`: host configuration from config file
/// `socket`: the socket what tokio TcpListener accepted
pub async fn graceful_shutdown(host: &SettingHost, socket: (TcpStream, SocketAddr)) {
    let (stream, addr) = socket;
    let io = TokioIo::new(stream);

    // Use keep_alive in config for incoming connections to the server.
    // use process_timeout in config for processing the final request and graceful shutdown.
    let connection_timeouts = [
        Duration::from_secs(host.keep_alive.into()),
        Duration::from_secs(host.process_timeout.into()),
    ];

    // service_fn
    let service = move |req| async move {
        let start_time = time::Instant::now();
        let res = handle_connection(req, host).await;
        let response = match res {
            Ok(res) => res,
            Err(Error::NotFound(err)) => {
                warn!("{err}");
                not_found()
            }
            _ => internal_server_error(),
        };
        let end_time = (Instant::now() - start_time).as_micros() as f32;
        let end_time = end_time / 1000_f32;
        info!("done {} {:.3}ms", addr, end_time);
        anyhow::Ok(response)
    };

    let conn = http1::Builder::new().serve_connection(io, service_fn(service));
    let mut conn = pin!(conn);
    // Iterate the timeouts.  Use tokio::select! to wait on the
    // result of polling the connection itself,
    // and also on tokio::time::sleep for the current timeout duration.
    for (i, sleep_duration) in connection_timeouts.iter().enumerate() {
        debug!("iter {} duration {:?}", i, sleep_duration);
        select! {
            res = conn.as_mut() => {
                match res {
                    Ok(_) => {}
                    Err(err)
                        if err.source().is_some()
                            && err
                                .source()
                                .unwrap()
                                .downcast_ref::<std::io::Error>()
                                .unwrap_or(&std::io::Error::new(NotFound, &Error::Empty))
                                .kind()
                                == NotConnected =>
                    {
                        // The client closed connection
                        debug!("client closed connection")
                    }
                    Err(err) => {
                        error!("handle connection {:?}", err);
                    }
                }
            }
            _ = tokio::time::sleep(*sleep_duration) => {
                debug!("iter = {} got timeout_interval, calling conn.graceful_shutdown", i);
                conn.as_mut().graceful_shutdown();
            }
        }
    }
}

/// Connection handler in service_fn
pub async fn handle_connection(
    req: Request<Incoming>,
    host: &SettingHost,
) -> Result<Response<CandyBody<Bytes>>> {
    use Error::*;

    let req_path = req.uri().path();
    let req_method = req.method();

    // find route path
    let not_found_err = NotFound(format!("resource {} not found", &req_path).into());
    let (router, assets_path) = find_route(req_path, &host.route_map)?;

    // find resource local file path
    let mut path = None;
    for index in host.index.iter() {
        let p = parse_assets_path(assets_path, &router.root, index);
        if Path::new(&p).exists() {
            path = Some(p);
            break;
        }
    }
    let path = match path {
        Some(p) => p,
        None => {
            return Err(not_found_err);
        }
    };

    // build the response for client
    let res = Response::builder();

    // http method handle
    let res = match *req_method {
        Method::GET => handle_get(req, res, &path).await?,
        Method::POST => handle_get(req, res, &path).await?,
        // Return the 404 Not Found for other routes.
        _ => {
            return Err(not_found_err);
        }
    };
    Ok(res)
}
