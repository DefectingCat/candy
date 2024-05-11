use async_compression::tokio::write::{BrotliEncoder, DeflateEncoder, GzipEncoder, ZstdEncoder};
use std::io::Result;
use tokio::io::AsyncWriteExt;

pub enum CompressType {
    Zstd,
    Gzip,
    Deflate,
    Brotli,
}

pub enum Encoder {
    Zstd(Box<ZstdEncoder<Vec<u8>>>),
    Gzip(Box<GzipEncoder<Vec<u8>>>),
    Deflate(Box<DeflateEncoder<Vec<u8>>>),
    Brotli(Box<BrotliEncoder<Vec<u8>>>),
}

impl Encoder {
    pub async fn encode(self, in_data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Encoder::Zstd(mut encoder) => {
                encoder.write_all(in_data).await?;
                encoder.shutdown().await?;
                Ok(encoder.into_inner())
            }
            Encoder::Gzip(mut encoder) => {
                encoder.write_all(in_data).await?;
                encoder.shutdown().await?;
                Ok(encoder.into_inner())
            }
            Encoder::Deflate(mut encoder) => {
                encoder.write_all(in_data).await?;
                encoder.shutdown().await?;
                Ok(encoder.into_inner())
            }
            Encoder::Brotli(mut encoder) => {
                encoder.write_all(in_data).await?;
                encoder.shutdown().await?;
                Ok(encoder.into_inner())
            }
        }
    }
}

pub async fn compress(compress_type: CompressType, in_data: &[u8]) -> Result<Vec<u8>> {
    use CompressType::*;

    let buffer = Vec::with_capacity(in_data.len());
    let encoder = match compress_type {
        Zstd => Encoder::Zstd(Box::new(ZstdEncoder::new(buffer))),
        Gzip => Encoder::Gzip(Box::new(GzipEncoder::new(buffer))),
        Deflate => Encoder::Deflate(Box::new(DeflateEncoder::new(buffer))),
        Brotli => Encoder::Brotli(Box::new(BrotliEncoder::new(buffer))),
    };
    encoder.encode(in_data).await
}
