//! 统一错误类型：将业务错误码、数据库异常、验证失败等映射为 HTTP 响应。
//!
//! `AppError` 实现 `IntoResponse`，可在 handler 中直接用 `?` 传播，自动转为 `{code, msg, data}` 格式。

use crate::response::Result;
use crate::result_code::ResultCode;
use axum::{http::StatusCode, response::IntoResponse, Json};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{1:?}")]
    Business(ResultCode, Option<String>),

    #[error("parameter validation failed: {0}")]
    Validation(String),

    #[error("database error: {0}")]
    Database(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<validator::ValidationErrors> for AppError {
    fn from(e: validator::ValidationErrors) -> Self {
        AppError::Validation(e.to_string())
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(e: redis::RedisError) -> Self {
        AppError::Database(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let result: Result<()> = match self {
            AppError::Business(code, msg) => Result::failed(code, msg),
            AppError::Validation(msg) => {
                Result::failed(ResultCode::UserRequestParameterError, Some(msg))
            }
            AppError::Database(msg) => {
                tracing::error!("database error: {msg}");
                Result::failed(ResultCode::DatabaseExecutionError, Some(msg))
            }
            AppError::Other(e) => {
                tracing::error!("{e:#}");
                Result::failed(ResultCode::SystemError, None)
            }
        };
        (StatusCode::OK, Json(result)).into_response()
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;
