use crate::error::{Error, Result};

use futures_util::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Bytes, Frame},
    Response, StatusCode,
};

use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::error;

pub type CandyBody<T, E = Error> = BoxBody<T, E>;

// pub fn default_headers() {}

/// Open local file by path, then use `ReaderStream` to stream to client
pub async fn stream_file(path: &str) -> Result<Response<CandyBody<Bytes>>> {
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
pub fn not_found() -> Response<CandyBody<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(NOT_FOUND.into()).map_err(|e| match e {}).boxed())
        .unwrap()
}
