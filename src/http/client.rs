use std::str::FromStr;

use anyhow::Context;
use bytes::Bytes;
use http::{request::Parts, Request, Response, Uri};
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::Incoming;
use hyper_rustls::ConfigBuilderExt;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};

use crate::error::Error;

const MAX_REDIRECTS: usize = 10;

/// Get http response
///
/// ## Example
///
/// ```rust
/// use candy::http::client::get_inner;
///
/// let res = get_inner("https://www.google.com").await.unwrap();
/// ```
///
/// ## Return
///
/// `anyhow::Result<Response<Incoming>>`
pub async fn get_inner(
    url: Uri,
    parts: Parts,
    body: Bytes,
) -> anyhow::Result<(Response<Incoming>, Parts, Bytes)> {
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
    // let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build(https);
    let mut req: Request<Full<Bytes>> = hyper::Request::builder()
        .method(parts.method.clone())
        .uri(url)
        .body(Full::from(body.clone()))
        .with_context(|| "request builder")?;
    req.headers_mut().extend(parts.clone().headers);

    let res = client.request(req).await?;
    Ok((res, parts, body))
}

/// Get http response Body
/// And follow redirects
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
pub async fn get(url: Uri, parts: Parts, body: Bytes) -> anyhow::Result<BoxBody<Bytes, Error>> {
    let mut redirects = 0;

    let (mut res, mut parts, mut body) = get_inner(url, parts, body).await?;
    while (res.status() == 301 || res.status() == 302) && redirects < MAX_REDIRECTS {
        redirects += 1;
        let location = res
            .headers()
            .get("location")
            .ok_or(Error::MissingHeader("location"))
            .with_context(|| "missing header location")?
            .to_str()
            .with_context(|| "failed to convert header value to str")?
            .to_string();
        let url = Uri::from_str(&location).with_context(|| "failed to convert str to url")?;
        (res, parts, body) = get_inner(url, parts, body).await?;
    }

    let res = res.map_err(Error::HyperError).boxed();
    Ok(res)
}
