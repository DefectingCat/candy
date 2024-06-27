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
    http::{handle_get, handle_not_found, internal_server_error, not_found, CandyBody},
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
use tokio::net::{TcpListener, TcpStream};

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

            tokio::select! {
                _ = graceful.shutdown() => {
                    info!("Gracefully shutdown!");
                },
                _ = tokio::time::sleep(Duration::from_secs(self.keep_alive.into())) => {
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
pub async fn handle_service(
    req: &Request<Incoming>,
    host: &'static SettingHost,
) -> Result<Response<CandyBody<Bytes>>> {
    use Error::*;

    let req_path = req.uri().path();

    // find route path
    let (router, assets_path) = find_route(req_path, &host.route_map)?;

    // build the response for client
    let mut res = Response::builder();
    let headers = res
        .headers_mut()
        .ok_or(InternalServerError(anyhow!("build response failed")))?;
    let server = format!("{}/{}", NAME, VERSION);
    headers.insert("Server", server.parse()?);
    // config headers overrite
    if let Some(c_headers) = &host.headers {
        for (k, v) in c_headers {
            headers.insert(k.as_str(), v.parse()?);
        }
    }

    // reverse proxy
    if router.proxy_pass.is_some() {
        handle_proxy(router, assets_path, req, res).await
    } else {
        // static file
        handle_file(router, assets_path, req, res).await
    }
}

/// Handle reverse proxy
///
/// Only use with the `proxy_pass` field in config
/// TODO: add x-proxy-server header
/// TODO: follow redirect
#[instrument(level = "debug")]
async fn handle_proxy(
    router: &SettingRoute,
    assets_path: &str,
    req: &Request<Incoming>,
    res: Builder,
) -> Result<Response<CandyBody<Bytes>>> {
    // check on outside
    let proxy = router.proxy_pass.as_ref().ok_or(Error::Empty)?;
    let path_query = req.uri().query().unwrap_or(assets_path);

    let uri: hyper::Uri = format!("{}{}", proxy, path_query)
        .parse()
        .with_context(|| format!("parse proxy uri failed: {}", proxy))?;
    match uri.scheme_str() {
        Some("http") | Some("https") => {}
        _ => {
            return Err(Error::InternalServerError(anyhow!(
                "proxy uri scheme error: {}",
                uri
            )));
        }
    }

    let host = uri.host().ok_or(Error::InternalServerError(anyhow!(
        "proxy pass host incorrect"
    )))?;
    let port = uri.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    // TODO: TcpStream timeout
    let stream = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("connect to {} failed", addr))?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|err| {
            error!("cannot handshake with {}: {}", addr, err);
            anyhow!("{err}")
        })?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let authority = uri
        .authority()
        .ok_or(anyhow!("proxy pass uri authority incorrect"))?;

    let path = uri.path();
    let req = Request::builder()
        .uri(path)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    let client_res = sender
        .send_request(req)
        .await
        .with_context(|| "send request failed")?;
    let client_body = client_res.map_err(Error::HyperError).boxed();
    let res_body = res.body(client_body)?;
    Ok(res_body)
}

/// Handle static files,
/// try find static file from local path
///
/// Only use with the `proxy_pass` field not in config
async fn handle_file(
    router: &SettingRoute,
    assets_path: &str,
    req: &Request<Incoming>,
    res: Builder,
) -> Result<Response<CandyBody<Bytes>>> {
    let req_method = req.method();

    // find resource local file path
    let mut path = None;
    for index in router.index.iter() {
        if let Some(root) = &router.root {
            let p = parse_assets_path(assets_path, root, index);
            if Path::new(&p).exists() {
                path = Some(p);
                break;
            }
        }
    }
    let path = match path {
        Some(p) => p,
        None => {
            return handle_not_found(req, res, router, "").await;
        }
    };

    // http method handle
    let res = match *req_method {
        Method::GET => handle_get(req, res, &path).await?,
        Method::POST => handle_get(req, res, &path).await?,
        // Return the 404 Not Found for other routes.
        _ => {
            if let Some(err_page) = &router.error_page {
                let res = res.status(err_page.status);
                handle_get(req, res, &err_page.page).await?
            } else {
                not_found()
            }
        }
    };
    Ok(res)
}
