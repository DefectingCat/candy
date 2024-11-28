use bytes::Bytes;
use http::Uri;
use http_body_util::{combinators::BoxBody, BodyExt, Empty};
use hyper_rustls::ConfigBuilderExt;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};

use crate::error::Error;

/// Get http response Body
///
/// ## Arguments
///
/// `url`: http url
///
/// ## Example
///
/// ```rust
/// use candy::http::client::get;
///
/// let body = get("https://www.google.com").await.unwrap();
/// ```
///
/// ## Return
///
/// `anyhow::Result<Bytes>`
pub async fn get(url: Uri) -> anyhow::Result<BoxBody<Bytes, Error>> {
    // let _ = rustls::crypto::ring::default_provider().install_default();
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Prepare the TLS client config
    // Default TLS client config with native roots
    let tls = rustls::ClientConfig::builder()
        .with_native_roots()?
        .with_no_client_auth();

    // Prepare the HTTPS connector
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls)
        .https_or_http()
        .enable_http1()
        .build();

    // Build the hyper client from the HTTPS connector.
    let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);

    let res = client.get(url).await?.map_err(Error::HyperError).boxed();
    Ok(res)
}
