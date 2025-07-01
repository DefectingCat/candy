use std::fmt::Display;

use crate::consts::{NAME, VERSION};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use const_format::formatcp;
use serde_repr::*;
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum RouteError {
    // Common errors
    #[error("{0}")]
    Any(#[from] anyhow::Error),
    #[error("{0}")]
    Infallible(#[from] std::convert::Infallible),

    // Route errors
    #[error("route not found")]
    RouteNotFound(),
    #[error("internal error")]
    InternalError(),
    #[error("bad request")]
    BadRequest(),
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u16)]
pub enum ErrorCode {
    Normal = 200,
    InternalError = 500,
    NotFound = 404,
    BadRequest = 400,
}

/// Normal error message
const SERVER_ERROR_STR: &str = formatcp!(
    r#"Internal Server Error
{NAME} v{VERSION}
Powered by RUA
"#
);

/// Not found error message
const NOT_FOUND_STR: &str = formatcp!(
    r#"Resource Not Found
{NAME} v{VERSION}
Powered by RUA
"#
);

const BAD_REQUEST_STR: &str = formatcp!(
    r#"Bad Request
{NAME} v{VERSION}
Powered by RUA
"#
);

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorCode::*;

        let res = match self {
            Normal => "",
            InternalError => SERVER_ERROR_STR,
            NotFound => NOT_FOUND_STR,
            BadRequest => BAD_REQUEST_STR,
        };
        f.write_str(res)?;
        Ok(())
    }
}

/// Log and return INTERNAL_SERVER_ERROR
fn log_internal_error<T: Display>(err: T) -> (StatusCode, String) {
    use ErrorCode::*;

    error!("{err}");
    (StatusCode::INTERNAL_SERVER_ERROR, InternalError.to_string())
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for RouteError {
    fn into_response(self) -> Response {
        use RouteError::*;

        let (status_code, err_message) = match self {
            Any(err) => log_internal_error(err),
            RouteNotFound() => (StatusCode::NOT_FOUND, ErrorCode::NotFound.to_string()),
            InternalError() => (StatusCode::NOT_FOUND, ErrorCode::InternalError.to_string()),
            // Infallible(infallible) => todo!(),
            BadRequest() => (StatusCode::NOT_FOUND, ErrorCode::BadRequest.to_string()),
        };
        (status_code, err_message).into_response()
    }
}

pub type RouteResult<T, E = RouteError> = Result<T, E>;
