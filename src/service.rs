use crate::{
    error::{Error, Result},
    utils::find_static_path,
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
use tokio::{fs::File, net::TcpListener};
use tokio_util::io::ReaderStream;
use tracing::{error, warn};

use crate::config::SettingHost;

type CandyBody<T, E = Error> = BoxBody<T, E>;

impl SettingHost {
    pub fn mk_server(&'static self) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let addr = format!("{}:{}", self.ip, self.port);
        #[allow(unreachable_code)]
        async move {
            let listener = TcpListener::bind(addr).await?;
            loop {
                let (stream, _) = listener.accept().await?;
                let io = TokioIo::new(stream);

                tokio::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(|req| async move {
                                let res = handle_connection(req, self).await;
                                let response = match res {
                                    Ok(res) => res,
                                    Err(Error::NotFound(err)) => {
                                        warn!("{err}");
                                        not_found()
                                    }
                                    _ => todo!(),
                                };
                                anyhow::Ok(response)
                            }),
                        )
                        .await
                    {
                        error!("Serving connection: {:?}", err);
                    };
                });
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

    let router = host
        .route_map
        .get(req_path)
        .ok_or(Error::NotFound(format!("route {} not found", req_path)))?;
    let assets_path = find_static_path(req_path, &router.location).unwrap_or("/");
    let _index = &host.index;
    let path = if assets_path.ends_with('/') {
        format!("{}{}{}", router.root, assets_path, host.index[0])
    } else {
        format!("{}{}/{}", router.root, assets_path, host.index[0])
    };
    dbg!(&path);

    let res = match req_method {
        &Method::GET => handle_file(&path).await?,
        // Return the 404 Not Found for other routes.
        _ => not_found(),
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
