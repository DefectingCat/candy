use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use serde_repr::*;
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum RouteError {
    #[error("{0}")]
    Any(#[from] anyhow::Error),
    #[error("{0}")]
    Infallible(#[from] std::convert::Infallible),
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u16)]
pub enum ErrorCode {
    Normal = 200,
    InternalError = 1000,
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorCode::*;

        let res = match self {
            Normal => "",
            InternalError => "服务器内部错误",
        };
        f.write_str(res)?;
        Ok(())
    }
}

/// Log and return INTERNAL_SERVER_ERROR
fn log_internal_error<T: Display>(err: T) -> (StatusCode, ErrorCode, String) {
    use ErrorCode::*;

    error!("{err}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        InternalError,
        "internal server error".to_string(),
    )
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for RouteError {
    fn into_response(self) -> Response {
        use RouteError::*;

        let (status_code, code, err_message) = match self {
            Any(err) => log_internal_error(err),
        };
        let body = Json(json!({
            "code": code,
            "message": code.to_string(),
            "error": err_message
        }));
        (status_code, body).into_response()
    }
}

pub type RouteResult<T, E = RouteError> = Result<T, E>;
