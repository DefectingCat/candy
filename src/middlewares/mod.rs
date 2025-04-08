use std::{fmt::Display, time::Duration};

use axum::{
    Router,
    body::Bytes,
    extract::Request,
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::TraceLayer;
use tracing::{Span, error, info, info_span};

use crate::{consts::{NAME, VERSION}, http::AppState};

/// Middleware for adding version information to each response's headers.
///
/// This middleware takes an incoming `Request` and a `Next` handler, which represents the
/// subsequent middleware or route in the chain. It then asynchronously runs the next handler,
/// obtaining the response. After receiving the response, it appends two headers:
/// - "Server": The name of the server extracted from the Cargo package name.
/// - "S-Version": The version of the server extracted from the Cargo package version.
pub async fn add_version(req: Request<axum::body::Body>, next: Next) -> impl IntoResponse {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.append("Server", HeaderValue::from_static(NAME));
    headers.append("RUA-Version", HeaderValue::from_static(VERSION));
    res
}

/// Middleware for logging each request.
///
/// This middleware will calculate each request latency
/// and add request's information to each info_span.
pub fn logging_route(router: Router) -> Router {
    let make_span = |req: &Request<_>| {
        let unknown = &HeaderValue::from_static("Unknown");
        let empty = &HeaderValue::from_static("");
        let headers = req.headers();
        let ua = headers
            .get("User-Agent")
            .unwrap_or(unknown)
            .to_str()
            .unwrap_or("Unknown");
        let host = headers.get("Host").unwrap_or(empty).to_str().unwrap_or("");
        info_span!("HTTP", method = ?req.method(), host, uri = ?req.uri(), ua)
    };

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(make_span)
        .on_request(|_req: &Request<_>, _span: &Span| {})
        .on_response(|res: &Response, latency: Duration, _span: &Span| {
            info!("{}", format_latency(latency, res.status()));
        })
        .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {})
        .on_eos(|_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {})
        .on_failure(
            |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                error!("{}", format_latency(latency, error));
            },
        );

    router.layer(trace_layer)
}

/// Format request latency and status message
/// return a string
fn format_latency(latency: Duration, status: impl Display) -> String {
    let micros = latency.as_micros();
    let millis = latency.as_millis();
    if micros >= 1000 {
        format!("{} {}ms", status, millis)
    } else {
        format!("{} {}μs", status, micros)
    }
}
