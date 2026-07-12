use axum::{http::StatusCode, response::IntoResponse, Json};
use crate::common::enums::ResultCode;
use crate::framework::web::response::Result;

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
