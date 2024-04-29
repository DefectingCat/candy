use std::time::UNIX_EPOCH;

use crate::{
    error::{Error, Result},
    get_cache,
};

use futures_util::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Bytes, Frame},
    HeaderMap, Response, StatusCode,
};

use tokio::{fs::File, io::AsyncReadExt};
use tokio_util::{bytes::BytesMut, io::ReaderStream};
use tracing::error;

pub type CandyBody<T, E = Error> = BoxBody<T, E>;

// pub fn default_headers() {}

/// Open local file and check last modified time,
/// Then determine stream file or use cache file
///
/// ## Arguments
///
/// `path`: local file path
pub async fn handle_file(path: &str, headers: &mut HeaderMap) -> Result<CandyBody<Bytes>> {
    // Open file for reading
    let file = File::open(path).await;
    let mut file = match file {
        Ok(f) => f,
        Err(err) => {
            error!("Unable to open file {err}");
            return Err(Error::NotFound(format!("path not found {}", path)));
        }
    };
    let matedata = file.metadata().await?;
    let size = matedata.len();
    let last_modified = matedata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
    // headers.insert("Last-", last_modified.into());
    headers.insert("Etag", format!("{last_modified}-{size}").parse()?);

    /* {
        let cache = get_cache().read()?;
        match cache.get(path) {
            Some(time) => {
                // dbg!(time, last_modified);
            }
            None => {
                drop(cache);
                let mut cache = get_cache().write()?;
                cache.insert(path.to_string(), last_modified);
            }
        }
    } */

    read_file(&mut file, size).await
}

/// Open then use `ReaderStream` to stream to client.
/// Stream a file more suit large file, but its slower than read file to memory.
pub async fn stream_file(file: File) -> Result<CandyBody<Bytes>> {
    // Wrap to a tokio_util::io::ReaderStream
    let reader_stream = ReaderStream::new(file);
    // Convert to http_body_util::BoxBody
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    // let boxed_body = stream_body.map_err(|e| Error::IoError(e)).boxed();
    let boxed_body = BodyExt::map_err(stream_body, Error::Io).boxed();
    Ok(boxed_body)
}

/// Open local file to memory
pub async fn read_file(file: &mut File, size: u64) -> Result<CandyBody<Bytes>> {
    let mut buffer = vec![0u8; size.try_into()?];
    file.read_exact(&mut buffer[..]).await?;
    let bytes = Bytes::from_iter(buffer);
    let body = Full::new(bytes).map_err(|e| match e {}).boxed();
    Ok(body)
}

// HTTP status code 404
static NOT_FOUND: &[u8] = b"Not Found";
pub fn not_found() -> Response<CandyBody<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(NOT_FOUND.into()).map_err(|e| match e {}).boxed())
        .unwrap()
}

static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
pub fn internal_server_error() -> Response<CandyBody<Bytes>> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(
            Full::new(INTERNAL_SERVER_ERROR.into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}
