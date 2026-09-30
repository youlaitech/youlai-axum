//! 文件上传/删除的 HTTP 接口处理函数。

use axum::{
    extract::{Multipart, Query, State},
};
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::modules::file::dto::{DeleteFileQuery, FileInfo};
use crate::modules::file::service::{
    delete_s3_object, extract_s3_key, upload_to_s3,
};
use crate::response::Result;
use crate::result_code::ResultCode;
use crate::state::AppState;

/// 上传文件：按 file.type 走 S3(RustFS) 或本地磁盘，统一做扩展名与大小校验
pub async fn upload_file(
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
        let content_type = field.content_type().map(|s| s.to_string());
        let data = field.bytes().await.map_err(|e| {
            AppError::Business(ResultCode::UploadFileException, Some(e.to_string()))
        })?;

        let ext = PathBuf::from(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_string();

        // 扩展名白名单校验（为空不限制）
        let allowed = cfg
            .upload
            .allowed_extensions
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if !allowed.is_empty() && !allowed.contains(&ext.to_lowercase()) {
            return Err(AppError::Business(
                ResultCode::UploadFileException,
                Some(format!("不支持的文件类型: .{ext}")),
            ));
        }

        // 单文件大小校验（0 表示不限制）
        let max_mb = cfg.upload.max_file_size_mb;
        if max_mb > 0 && data.len() as u64 > max_mb * 1024 * 1024 {
            return Err(AppError::Business(
                ResultCode::UploadFileException,
                Some(format!("文件大小超过限制: {max_mb}MB")),
            ));
        }

        let date_dir = Utc::now().format("%Y%m%d").to_string();
        let stored_name = format!("{}.{}", Uuid::new_v4(), ext);
        let object_key = format!("{date_dir}/{stored_name}");

        let url = match cfg.r#type.as_str() {
            "s3" => upload_to_s3(&cfg.s3, &object_key, data.to_vec(), content_type).await?,
            "local" => {
                let abs_path = PathBuf::from(&cfg.local.path).join(&object_key);
                if let Some(parent) = abs_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        AppError::Business(ResultCode::UploadFileException, Some(e.to_string()))
                    })?;
                }
                tokio::fs::write(&abs_path, &data).await.map_err(|e| {
                    AppError::Business(ResultCode::UploadFileException, Some(e.to_string()))
                })?;
                format!("{}/{}", cfg.local.base_url.trim_end_matches('/'), object_key)
            }
            other => {
                return Err(AppError::Business(
                    ResultCode::UploadFileException,
                    Some(format!("未实现的存储类型: {other}")),
                ))
            }
        };

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

/// 删除文件：按 file.type 走 S3(RustFS) 或本地磁盘
pub async fn delete_file(
    State(state): State<AppState>,
    Query(q): Query<DeleteFileQuery>,
) -> AppResult<axum::Json<Result<()>>> {
    let cfg = &state.settings.file;
    match cfg.r#type.as_str() {
        "s3" => {
            let key = extract_s3_key(&cfg.s3, &q.file_path);
            delete_s3_object(&cfg.s3, &key).await?;
        }
        "local" => {
            let rel = q
                .file_path
                .trim_start_matches(&cfg.local.base_url)
                .trim_start_matches('/');
            let root = PathBuf::from(&cfg.local.path);
            let abs = root.join(rel);
            if abs.exists() {
                // 防目录穿越：规范化后的目标必须仍在存储根目录内
                let root_abs = root.canonicalize().map_err(|e| {
                    AppError::Business(ResultCode::DeleteFileException, Some(e.to_string()))
                })?;
                let target = abs.canonicalize().map_err(|e| {
                    AppError::Business(ResultCode::DeleteFileException, Some(e.to_string()))
                })?;
                if !target.starts_with(&root_abs) {
                    return Err(AppError::Business(
                        ResultCode::DeleteFileException,
                        Some("文件路径不合法".into()),
                    ));
                }
                tokio::fs::remove_file(&target).await.map_err(|e| {
                    AppError::Business(ResultCode::DeleteFileException, Some(e.to_string()))
                })?;
            }
        }
        other => {
            return Err(AppError::Business(
                ResultCode::DeleteFileException,
                Some(format!("未实现的存储类型: {other}")),
            ))
        }
    }
    Ok(axum::Json(Result::success_empty()))
}
