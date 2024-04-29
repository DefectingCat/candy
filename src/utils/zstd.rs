use async_compression::tokio::write::{ZstdDecoder, ZstdEncoder};
use std::io::Result;
use tokio::io::AsyncWriteExt;

pub async fn compress(in_data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZstdEncoder::new(Vec::new());
    encoder.write_all(in_data).await?;
    encoder.shutdown().await?;
    Ok(encoder.into_inner())
}

pub async fn decompress(in_data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZstdDecoder::new(Vec::new());
    decoder.write_all(in_data).await?;
    decoder.shutdown().await?;
    Ok(decoder.into_inner())
}
