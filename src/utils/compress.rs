use async_compression::tokio::bufread::{BrotliEncoder, DeflateEncoder, GzipEncoder, ZstdEncoder};
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::{Bytes, Frame};
use tokio::io::{AsyncBufRead, BufReader};
use tokio_util::io::ReaderStream;

use crate::{error::Error, http::CandyBody};

pub enum CompressType {
    Zstd,
    Gzip,
    Deflate,
    Brotli,
}

macro_rules! encode {
    ($encoder:ident, $file:ident) => {{
        let encoder_stream = $encoder::new($file);
        let reader_stream = ReaderStream::new(encoder_stream);
        let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
        let boxed_body = BodyExt::map_err(stream_body, Error::Io).boxed();
        boxed_body
    }};
}

pub fn stream_compress<R>(compress_type: CompressType, reader: R) -> CandyBody<Bytes>
where
    R: AsyncBufRead + Sync + Send + 'static,
{
    use CompressType::*;

    let file_reader = BufReader::new(reader);

    match compress_type {
        Zstd => {
            encode!(ZstdEncoder, file_reader)
        }
        Gzip => encode!(GzipEncoder, file_reader),
        Deflate => encode!(DeflateEncoder, file_reader),
        Brotli => encode!(BrotliEncoder, file_reader),
    }
}
