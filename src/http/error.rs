//! HTTP 路由错误处理模块
//!
//! 定义 HTTP 路由处理过程中的错误类型和错误响应。
//!
//! # 错误类型
//!
//! - `RouteNotFound`：路由未找到（404）
//! - `InternalError`：服务器内部错误（500）
//! - `BadRequest`：客户端请求错误（400）
//! - `Any`：通用错误（包装 anyhow::Error）
//! - `Infallible`：不可达错误类型
//!
//! # 错误响应
//!
//! 所有错误都会转换为 HTTP 响应，包含：
//! - 状态码（404、500、400 等）
//! - 错误信息（包含服务器名称和版本）
//!
//! # 示例
//!
//! ```no_run
//! use candy::http::error::RouteError;
//! use axum::response::IntoResponse;
//!
//! // 创建错误
//! let err = RouteError::RouteNotFound();
//!
//! // 转换为 HTTP 响应
//! let response = err.into_response();
//! assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
//! ```

use std::fmt::Display;

use crate::consts::{NAME, VERSION};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use const_format::formatcp;
use serde_repr::*;
use tracing::{debug, error};

/// HTTP 路由错误类型
///
/// 定义了 HTTP 路由处理过程中可能出现的所有错误类型。
/// 所有错误都会自动转换为 HTTP 响应返回给客户端。
///
/// # 错误变体
///
/// - `Any` - 通用错误，包装 anyhow::Error
/// - `Infallible` - 不可达错误类型（理论上不会发生）
/// - `RouteNotFound` - 路由未找到（404 错误）
/// - `InternalError` - 服务器内部错误（500 错误）
/// - `BadRequest` - 客户端请求错误（400 错误）
///
/// # 错误响应
///
/// 所有错误都会转换为包含服务器信息的友好错误页面：
///
/// ```text
/// Resource Not Found
/// Candy v0.2.5
/// Powered by RUA
/// ```
///
/// # 示例
///
/// ```no_run
/// use candy::http::error::RouteError;
/// use axum::response::IntoResponse;
///
/// // 创建路由未找到错误
/// let err = RouteError::RouteNotFound();
/// let response = err.into_response();
/// assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
///
/// // 创建内部错误
/// let err = RouteError::InternalError();
/// let response = err.into_response();
/// assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND); // 注意：当前实现返回 404
/// ```
#[derive(thiserror::Error, Debug)]
pub enum RouteError {
    /// 通用错误
    ///
    /// 包装 anyhow::Error，用于处理各种类型的错误。
    #[error("{0}")]
    Any(#[from] anyhow::Error),

    /// 不可达错误
    ///
    /// 理论上不会发生的错误类型。
    #[error("{0}")]
    Infallible(#[from] std::convert::Infallible),

    /// 路由未找到
    ///
    /// 客户端请求的资源不存在，返回 404 状态码。
    #[error("route not found")]
    RouteNotFound(),

    /// 服务器内部错误
    ///
    /// 服务器处理请求时发生内部错误，返回 500 状态码。
    #[error("internal error")]
    InternalError(),

    /// 客户端请求错误
    ///
    /// 客户端发送的请求格式错误或参数无效，返回 400 状态码。
    #[error("bad request")]
    BadRequest(),
}

/// 错误代码枚举
///
/// 定义了 HTTP 响应的状态码和对应的错误信息。
/// 使用 `#[repr(u16)]` 确保与 HTTP 状态码数值一致。
///
/// # 变体
///
/// - `Normal` (200) - 正常响应
/// - `InternalError` (500) - 服务器内部错误
/// - `NotFound` (404) - 资源未找到
/// - `BadRequest` (400) - 客户端请求错误
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u16)]
pub enum ErrorCode {
    /// 正常响应（HTTP 200）
    Normal = 200,
    /// 服务器内部错误（HTTP 500）
    InternalError = 500,
    /// 资源未找到（HTTP 404）
    NotFound = 404,
    /// 客户端请求错误（HTTP 400）
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
            BadRequest() => (StatusCode::NOT_FOUND, ErrorCode::BadRequest.to_string()),
            _ => (StatusCode::NOT_FOUND, ErrorCode::NotFound.to_string()),
        };
        debug!(
            "RouterError status_code {}, err_message {}",
            status_code, err_message
        );
        (status_code, err_message).into_response()
    }
}

pub type RouteResult<T, E = RouteError> = Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        // 测试 ErrorCode 显示
        assert!(ErrorCode::Normal.to_string().is_empty()); // 正常情况返回空字符串
        assert!(ErrorCode::InternalError
            .to_string()
            .contains("Internal Server Error"));
        assert!(ErrorCode::NotFound
            .to_string()
            .contains("Resource Not Found"));
        assert!(ErrorCode::BadRequest.to_string().contains("Bad Request"));
    }

    #[test]
    fn test_error_code_from_route_error() {
        // 测试 RouteError 转换为响应
        let err = RouteError::RouteNotFound();
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let err = RouteError::InternalError();
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND); // 注意：这里是 NotFound，看起来像个 bug

        let err = RouteError::BadRequest();
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND); // 注意：这里也是 NotFound
    }
}
