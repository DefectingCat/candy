use std::{
    path::Path,
    pin::pin,
    time::{self, Duration, Instant},
};

use crate::{
    error::{Error, Error::NotFound, Result},
    utils::{find_route, parse_assets_path},
};

use futures_util::{Future, TryStreamExt};
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Bytes, Frame, Incoming as IncomingBody},
    server::conn::http1,
    service::service_fn,
    Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use tokio::{fs::File, net::TcpListener, select};
use tokio_util::io::ReaderStream;
use tracing::{debug, error, info, warn};

use crate::config::SettingHost;

type CandyBody<T, E = Error> = BoxBody<T, E>;

impl SettingHost {
    pub fn mk_server(&'static self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        // Use a 5 second timeout for incoming connections to the server.
        // If a request is in progress when the 5 second timeout elapses,
        // use a 2 second timeout for processing the final request and graceful shutdown.
        let connection_timeouts = [Duration::from_secs(5), Duration::from_secs(2)];

        let addr = format!("{}:{}", self.ip, self.port);
        #[allow(unreachable_code)]
        async move {
            let listener = TcpListener::bind(&addr).await?;
            info!("host bind on {}", addr);
            loop {
                let (stream, addr) = listener.accept().await?;
                info!("accept from {}", &addr);
                let io = TokioIo::new(stream);

                let service = move |req| async move {
                    let start_time = time::Instant::now();
                    let res = handle_connection(req, self).await;
                    let response = match res {
                        Ok(res) => res,
                        Err(Error::NotFound(err)) => {
                            warn!("{err}");
                            not_found()
                        }
                        _ => todo!(),
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

    let res = match req_method {
        &Method::GET => handle_file(&path).await?,
        // Return the 404 Not Found for other routes.
        _ => {
            return Err(not_found_err);
        }
    };
    Ok(res)
}

async fn handle_file(path: &str) -> Result<Response<CandyBody<Bytes>>> {
    // Open file for reading
    let file = File::open(path).await;
    let file = match file {
        Ok(f) => f,
        Err(err) => {
            error!("Unable to open file {err}");
            return Ok(not_found());
        }
    };

    // Wrap to a tokio_util::io::ReaderStream
    let reader_stream = ReaderStream::new(file);
    // Convert to http_body_util::BoxBody
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    // let boxed_body = stream_body.map_err(|e| Error::IoError(e)).boxed();
    let boxed_body = BodyExt::map_err(stream_body, Error::Io).boxed();

    // Send response
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(boxed_body)?;

    Ok(response)
}

// HTTP status code 404
static NOT_FOUND: &[u8] = b"Not Found";
fn not_found() -> Response<CandyBody<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(NOT_FOUND.into()).map_err(|e| match e {}).boxed())
        .unwrap()
}
