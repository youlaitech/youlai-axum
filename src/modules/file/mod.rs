use axum::{
    extract::{Multipart, Query, State},
    routing::post,
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::common::enums::ResultCode;
use crate::core::state::AppState;
use crate::framework::web::error::{AppError, AppResult};
use crate::framework::web::response::Result;

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/files",
        post(upload_file).delete(delete_file),
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileQuery {
    pub file_path: String,
}

/// 上传文件（本地磁盘存储，按日期分目录）
async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<axum::Json<Result<FileInfo>>> {
    let cfg = &state.settings.file;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Business(ResultCode::UploadFileException, Some(e.to_string())))?
    {
        if field.name().unwrap_or_default() != "file" {
            continue;
        }
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload.bin".to_string());
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::Business(ResultCode::UploadFileException, Some(e.to_string())))?;

        let ext = PathBuf::from(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_string();
        let date_dir = Utc::now().format("%Y%m%d").to_string();
        let stored_name = format!("{}.{}", Uuid::new_v4(), ext);
        let rel_path = format!("{date_dir}/{stored_name}");
        let abs_path = PathBuf::from(&cfg.storage_path).join(&rel_path);

        if let Some(parent) = abs_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Business(ResultCode::UploadFileException, Some(e.to_string())))?;
        }
        tokio::fs::write(&abs_path, &data)
            .await
            .map_err(|e| AppError::Business(ResultCode::UploadFileException, Some(e.to_string())))?;

        let url = format!("{}/{}", cfg.base_url.trim_end_matches('/'), rel_path);
        return Ok(axum::Json(Result::success(FileInfo {
            name: file_name,
            url,
        })));
    }
    Err(AppError::Business(
        ResultCode::UploadFileException,
        Some("请求中未包含 file 字段".into()),
    ))
}

/// 删除文件（本地磁盘，路径为上传返回的 URL 或相对路径）
async fn delete_file(
    State(state): State<AppState>,
    Query(q): Query<DeleteFileQuery>,
) -> AppResult<axum::Json<Result<()>>> {
    let cfg = &state.settings.file;
    let rel = q
        .file_path
        .trim_start_matches(&cfg.base_url)
        .trim_start_matches('/')
        .to_string();
    let abs = PathBuf::from(&cfg.storage_path).join(&rel);
    if abs.exists() {
        tokio::fs::remove_file(&abs)
            .await
            .map_err(|e| AppError::Business(ResultCode::DeleteFileException, Some(e.to_string())))?;
    }
    Ok(axum::Json(Result::success_empty()))
}
