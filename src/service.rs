use std::{
    path::Path,
    pin::pin,
    time::{self, Duration, Instant},
};

use crate::{
    error::{Error, Result},
    http::{handle_file, internal_server_error, not_found, CandyBody},
    utils::{find_route, parse_assets_path},
};

use anyhow::anyhow;
use futures_util::Future;

use hyper::{
    body::{Bytes, Incoming as IncomingBody},
    server::conn::http1,
    service::service_fn,
    Method, Request, Response,
};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, select};

use tracing::{debug, error, info, warn};

use crate::config::SettingHost;

impl SettingHost {
    pub fn mk_server(&'static self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        #[allow(unreachable_code)]
        async move {
            let listener = TcpListener::bind(&addr).await?;
            info!("host bind on {}", addr);
            loop {
                let (stream, addr) = listener.accept().await?;
                info!("accept from {}", &addr);
                let io = TokioIo::new(stream);

                // Use keep_alive in config for incoming connections to the server.
                // use process_timeout in config for processing the final request and graceful shutdown.
                let connection_timeouts = [
                    Duration::from_secs(self.keep_alive.into()),
                    Duration::from_secs(self.process_timeout.into()),
                ];

                let service = move |req| async move {
                    let start_time = time::Instant::now();
                    let res = handle_connection(req, self).await;
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
                    info!("end {} {:.3}ms", addr, end_time);
                    anyhow::Ok(response)
                };

                let handler = tokio::spawn(async move {
                    let conn = http1::Builder::new().serve_connection(io, service_fn(service));
                    let mut conn = pin!(conn);
                    // Iterate the timeouts.  Use tokio::select! to wait on the
                    // result of polling the connection itself,
                    // and also on tokio::time::sleep for the current timeout duration.
                    for (i, sleep_duration) in connection_timeouts.iter().enumerate() {
                        debug!("iter {} duration {:?}", i, sleep_duration);
                        select! {
                            res = conn.as_mut() => {
                                res?;
                            }
                            _ = tokio::time::sleep(*sleep_duration) => {
                                info!("iter = {} got timeout_interval, calling conn.graceful_shutdown", i);
                                conn.as_mut().graceful_shutdown();
                            }
                        }
                    }
                    anyhow::Ok(())
                });
                if let Err(err) = handler.await {
                    error!("handle connection {:?}", err);
                }
            }
            anyhow::Ok(())
        }
    }
}

async fn handle_connection(
    req: Request<IncomingBody>,
    host: &SettingHost,
) -> Result<Response<CandyBody<Bytes>>> {
    use Error::*;

    let req_path = req.uri().path();
    let req_method = req.method();

    // find route path
    let not_found_err = NotFound(format!("resource {} not found", &req_path));
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
    let mut response = Response::builder();
    let headers = response.headers_mut().ok_or(InternalServerError(anyhow!(
        "build response failed, cannot get headser"
    )))?;
    headers.insert("Content-Type", "text/html".parse()?);

    // http method handle
    let res = match *req_method {
        Method::GET => response.body(handle_file(&path).await?)?,
        Method::POST => response.body(handle_file(&path).await?)?,
        // Return the 404 Not Found for other routes.
        _ => {
            return Err(not_found_err);
        }
    };
    Ok(res)
}
