use std::pin::Pin;

use crate::{
    error::{Error, Result},
    utils::find_static_path,
};
use futures_util::{Future, TryStreamExt};
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Bytes, Frame, Incoming as IncomingBody},
    server::conn::http1,
    service::Service,
    Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use tokio::{fs::File, net::TcpListener};
use tokio_util::io::ReaderStream;
use tracing::error;

use crate::config::SettingHost;

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
                    if let Err(err) = http1::Builder::new().serve_connection(io, self).await {
                        error!("Serving connection: {:?}", err);
                    };
                });
            }
            anyhow::Ok(())
        }
    }
}

impl Service<Request<IncomingBody>> for &SettingHost {
    type Response = Response<CandyBody<Bytes>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        Box::pin(mk_response(req))
    }
}

pub async fn mk_response(req: Request<IncomingBody>) -> Result<Response<CandyBody<Bytes>>> {
    // let route = &self.route;

    let req_path = req.uri().path();
    let req_method = req.method();

    let test = find_static_path(req_path, "/");
    dbg!(test);

    let res = match req_method {
        // Return the 404 Not Found for other routes.
        _ => not_found(),
    };

    Ok(res)
}

type CandyBody<T, E = Error> = BoxBody<T, E>;
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
