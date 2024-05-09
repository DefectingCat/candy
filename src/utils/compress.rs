use async_compression::tokio::write::{GzipEncoder, ZstdEncoder};
use std::io::Result;
use tokio::io::AsyncWriteExt;

pub enum CompressType {
    Zstd,
    Gzip,
}

pub enum Encoder {
    Zstd(ZstdEncoder<Vec<u8>>),
    Gzip(GzipEncoder<Vec<u8>>),
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
        }
    }
}

pub async fn compress(compress_type: CompressType, in_data: &[u8]) -> Result<Vec<u8>> {
    use CompressType::*;

    let encoder = match compress_type {
        Zstd => Encoder::Zstd(ZstdEncoder::new(vec![])),
        Gzip => Encoder::Gzip(GzipEncoder::new(vec![])),
    };
    encoder.encode(in_data).await
}
