use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
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
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u16)]
pub enum ErrorCode {
    Normal = 200,
    InternalError = 500,
    NotFound = 404,
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorCode::*;

        let res = match self {
            Normal => "",
            InternalError => "Internal Server Error",
            NotFound => "Resource Not Found",
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
            // route errors
            RouteNotFound() => (StatusCode::NOT_FOUND, ErrorCode::NotFound.to_string()),
        };
        // let body = Json(json!({
        //     "code": code,
        //     "message": code.to_string(),
        //     "error": err_message
        // }));
        (status_code, err_message).into_response()
    }
}

pub type RouteResult<T, E = RouteError> = Result<T, E>;
