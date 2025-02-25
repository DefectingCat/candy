use std::str::FromStr;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use http::{request::Parts, HeaderValue, Request, Response, Uri};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper_rustls::ConfigBuilderExt;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use tracing::debug;

use crate::error::Error;

const MAX_REDIRECTS: usize = 10;

/// Get http response
///
/// ## Arguments
///
/// `url`: http url
/// `parts`: http request parts
/// `body`: http request body
///
/// ## Return
///
/// `anyhow::Result<Response<Incoming>>`
pub async fn get_inner(url: Uri, parts: Parts, body: Bytes) -> anyhow::Result<Response<Incoming>> {
    // Set a process wide default crypto provider.
    #[cfg(feature = "ring")]
    let _ = rustls::crypto::ring::default_provider().install_default();
    #[cfg(feature = "aws-lc-rs")]
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
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build(https);
    let host_url = url.clone();
    let host = host_url.host().ok_or(Error::InternalServerError(anyhow!(
        "proxy pass host incorrect"
    )))?;
    let mut req: Request<Full<Bytes>> = hyper::Request::builder()
        .method(parts.method.clone())
        .uri(url)
        .body(Full::from(body))
        .with_context(|| "request builder")?;
    // Add client request headers to request, and remove host header
    req.headers_mut().extend(parts.headers);
    req.headers_mut()
        .insert("host", HeaderValue::from_str(host)?);

    let res = client.request(req).await?;
    Ok(res)
}

/// Get http response Body
/// And follo redirects
///
/// ## Arguments
///
/// `url`: http url
/// `parts`: http request parts
/// `body`: http request body
///
/// ## Return
///
/// `anyhow::Result<Response<Incoming>>`
pub async fn get(url: Uri, parts: Parts, body: Bytes) -> anyhow::Result<Response<Incoming>> {
    let mut redirects = 0;

    let mut res = get_inner(url, parts.clone(), body.clone()).await?;
    while (res.status() == 301 || res.status() == 302) && redirects < MAX_REDIRECTS {
        let (parts_inner, body_inner) = (parts.clone(), body.clone());
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
        debug!("proxy redirect to {url}");
        res = get_inner(url, parts_inner, body_inner).await?;
    }

    debug!("get_inner response headers: {:?}", res.headers());
    Ok(res)
}
