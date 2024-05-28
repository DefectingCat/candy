use std::{
    error::Error as StdError,
    io::ErrorKind::{NotConnected, NotFound},
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

use anyhow::{anyhow, bail};
use futures_util::Future;
use http::{response::Builder, Method, Request, Response};
use http_body_util::{BodyExt, Empty, StreamBody};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use tokio::{
    io,
    net::{TcpListener, TcpStream},
    select,
};

use tokio_util::io::ReaderStream;
use tracing::{debug, error, info, warn};

impl SettingHost {
    pub fn mk_server(&'static self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        async move {
            let listener = TcpListener::bind(&addr).await?;
            info!("host bind on {}", addr);
            loop {
                let socket = listener.accept().await?;
                tokio::spawn(async move {
                    graceful_shutdown(self, socket).await;
                });
            }
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
pub async fn graceful_shutdown(host: &'static SettingHost, socket: (TcpStream, SocketAddr)) {
    let (stream, addr) = socket;
    let io = TokioIo::new(stream);

    // Use keep_alive in config for incoming connections to the server.
    // use process_timeout in config for processing the final request and graceful shutdown.
    let connection_timeout = Duration::from_secs(host.keep_alive.into());

    // service_fn
    let service = move |req: Request<Incoming>| async move {
        let start_time = time::Instant::now();
        let res = handle_connection(&req, host).await;
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
        let method = &req.method();
        let path = &req.uri().path();
        let version = &req.version();
        let res_status = response.status();
        info!(
            "\"{}\" {} {} {:?} {} {:.3}ms",
            addr, method, path, version, res_status, end_time
        );
        anyhow::Ok(response)
    };

    let conn = http1::Builder::new().serve_connection(io, service_fn(service));
    let mut conn = pin!(conn);
    // Iterate the timeouts.  Use tokio::select! to wait on the
    // result of polling the connection itself,
    // and also on tokio::time::sleep for the current timeout duration.
    select! {
        res = conn.as_mut() => {
            match res {
                Ok(_) => {
                    debug!("close connection");
                }
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
                    debug!("client closed connection");
                }
                Err(err) => {
                    error!("handle connection {:?}", err);
                }
            }
        }
        _ = tokio::time::sleep(connection_timeout) => {
            debug!("keep-alive timeout {}s, calling conn.graceful_shutdown", host.keep_alive);
            conn.as_mut().graceful_shutdown();
        }
    }
}

/// Connection handler in service_fn
pub async fn handle_connection(
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
async fn handle_proxy(
    router: &SettingRoute,
    assets_path: &str,
    req: &Request<Incoming>,
    res: Builder,
) -> Result<Response<CandyBody<Bytes>>> {
    // check on outside
    let proxy = router.proxy_pass.as_ref().ok_or(Error::Empty)?;
    let path_query = req.uri().query().unwrap_or(assets_path);

    let uri: hyper::Uri = format!("{}{}", proxy, path_query).parse()?;
    if uri.scheme_str() != Some("http") {
        return Err(Error::InternalServerError(anyhow!("")));
    }
    dbg!(&uri);

    let host = uri.host().ok_or(Error::InternalServerError(anyhow!("")))?;
    let port = uri.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|err| anyhow!("{err}"))?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let authority = uri.authority().unwrap().clone();

    let path = uri.path();
    let req = Request::builder()
        .uri(path)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    let res = sender.send_request(req).await?;

    // let stream_body = StreamBody::new(res);

    // Stream the body, writing each chunk to stdout as we get it
    // (instead of buffering and printing at the end).
    // while let Some(next) = res.frame().await {
    //     let frame = next?;
    //     if let Some(chunk) = frame.data_ref() {
    //         dbg!(chunk);
    //     }
    // }

    todo!();
    Ok(res.into())
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
