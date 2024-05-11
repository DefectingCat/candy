use std::{path::PathBuf, str::FromStr, time::UNIX_EPOCH};

use crate::{
    error::{Error, Result},
    get_settings,
    utils::compress::{compress, CompressType},
};

use anyhow::anyhow;
use http::response::Builder;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    Request, Response, StatusCode,
};

use tokio::{fs::File, io::AsyncReadExt};
use tracing::{debug, error};

pub type CandyBody<T, E = Error> = BoxBody<T, E>;

// pub fn default_headers() {}

/// Open local file and check last modified time,
/// Then determine stream file or use cache file
///
/// ## Arguments
///
/// `path`: local file path
pub async fn open_file(path: &str) -> Result<File> {
    // Open file for reading
    let file = File::open(path).await;
    let file = match file {
        Ok(f) => f,
        Err(err) => {
            error!("Unable to open file {err}");
            return Err(Error::NotFound(format!("path not found {}", path)));
        }
    };
    Ok(file)

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
}

/// Open then use `ReaderStream` to stream to client.
/// Stream a file more suitable for large file, but its slower than read file to memory.
// pub async fn stream_file(file: File) -> Result<CandyBody<Bytes>> {
//     // Wrap to a tokio_util::io::ReaderStream
//     let reader_stream = ReaderStream::new(file);
//     // Convert to http_body_util::BoxBody
//     let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
//     // let boxed_body = stream_body.map_err(|e| Error::IoError(e)).boxed();
//     let boxed_body = BodyExt::map_err(stream_body, Error::Io).boxed();
//     Ok(boxed_body)
// }

pub async fn read_file_bytes(file: &mut File, size: u64) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; size.try_into()?];
    file.read_exact(&mut buffer[..]).await?;
    Ok(buffer)
}

// Open local file to memory
// pub async fn read_file(file: &mut File, size: u64) -> Result<CandyBody<Bytes>> {
//     let bytes = read_file_bytes(file, size).await?;
//     let body = Full::new(bytes.into()).map_err(|e| match e {}).boxed();
//     Ok(body)
// }

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

// HTTP methods
/// handle http get method
/// read static file and check If-None-Match cache
pub async fn handle_get(
    req: Request<Incoming>,
    mut res: Builder,
    path: &str,
) -> Result<Response<CandyBody<Bytes>>> {
    use CompressType::*;
    use Error::*;

    let headers = res
        .headers_mut()
        .ok_or(InternalServerError(anyhow!("build response failed")))?;

    // file bytes
    let mut file = open_file(path).await?;
    // file info
    let metadata = file.metadata().await?;
    let size = metadata.len();
    let last_modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
    let etag = format!("{last_modified}-{size}");
    let extension = PathBuf::from_str(path).map_err(|err| InternalServerError(anyhow!(err)))?;
    let extension = extension
        .extension()
        .ok_or(InternalServerError(anyhow!("read file extension failed")))?;

    let settings = get_settings();
    let content_type = settings.types.get(
        extension
            .to_str()
            .ok_or(InternalServerError(anyhow!("read file extension failed")))?,
    );
    headers.insert(
        "Content-Type",
        content_type.unwrap_or(&settings.default_type).parse()?,
    );
    headers.insert("Etag", etag.parse()?);
    let file_buffer = read_file_bytes(&mut file, size).await?;

    // check cache
    let if_none_match = req.headers().get("If-None-Match");
    match if_none_match {
        Some(inm) if *inm == *etag => {
            let res = res.status(304);
            let body = Full::new(vec![].into()).map_err(|e| match e {}).boxed();
            return Ok(res.body(body)?);
        }
        _ => {}
    }

    // prepare compress
    let accept_encoding = req.headers().get("Accept-Encoding");
    let bytes = match accept_encoding {
        Some(accept) => {
            let accept = accept.to_str()?;
            debug!("Accept-Encoding {}", accept);
            match accept {
                str if str.contains("zstd") => {
                    headers.insert("Content-Encoding", "zstd".parse()?);
                    compress(Zstd, &file_buffer).await?
                }
                str if str.contains("gzip") => {
                    headers.insert("Content-Encoding", "gzip".parse()?);
                    compress(Gzip, &file_buffer).await?
                }
                str if str.contains("deflate") => {
                    headers.insert("Content-Encoding", "deflate".parse()?);
                    compress(Deflate, &file_buffer).await?
                }
                str if str.contains("br") => {
                    headers.insert("Content-Encoding", "br".parse()?);
                    compress(Brotli, &file_buffer).await?
                }
                _ => file_buffer,
            }
        }
        None => file_buffer,
    };

    let body = Full::new(bytes.into()).map_err(|e| match e {}).boxed();
    Ok(res.body(body)?)
}
