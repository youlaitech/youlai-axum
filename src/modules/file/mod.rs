//! 文件上传/删除模块：支持 MinIO（S3 兼容）和本地磁盘两种后端。
//!
//! 单文件大小、扩展名白名单通过配置控制；MinIO 模式返回完整对象 URL。

use axum::{
    extract::{Multipart, Query, State},
    routing::post,
    Router,
};
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::config::FileMinioConfig;
use crate::error::{AppError, AppResult};
use crate::response::Result;
use crate::result_code::ResultCode;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/files", post(upload_file).delete(delete_file))
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

/// 上传文件：按 file.type 走 MinIO 或本地磁盘，统一做扩展名与大小校验
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

        let url = if cfg.r#type == "minio" {
            upload_to_minio(&cfg.minio, &object_key, data.to_vec(), content_type).await?
        } else {
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

/// 删除文件：按 file.type 走 MinIO 或本地磁盘
async fn delete_file(
    State(state): State<AppState>,
    Query(q): Query<DeleteFileQuery>,
) -> AppResult<axum::Json<Result<()>>> {
    let cfg = &state.settings.file;
    if cfg.r#type == "minio" {
        let client = build_minio_client(&cfg.minio);
        let key = extract_minio_key(&cfg.minio, &q.file_path);
        client
            .delete_object()
            .bucket(&cfg.minio.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| AppError::Business(ResultCode::DeleteFileException, Some(e.to_string())))?;
    } else {
        let rel = q
            .file_path
            .trim_start_matches(&cfg.local.base_url)
            .trim_start_matches('/')
            .to_string();
        let abs = PathBuf::from(&cfg.local.path).join(&rel);
        if abs.exists() {
            tokio::fs::remove_file(&abs).await.map_err(|e| {
                AppError::Business(ResultCode::DeleteFileException, Some(e.to_string()))
            })?;
        }
    }
    Ok(axum::Json(Result::success_empty()))
}

/// 构建 MinIO（S3 兼容）客户端：路径风格寻址，region 固定占位
fn build_minio_client(cfg: &FileMinioConfig) -> S3Client {
    let creds = Credentials::new(
        cfg.access_key.clone(),
        cfg.secret_key.clone(),
        None,
        None,
        "youlai",
    );
    let conf = aws_sdk_s3::config::Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url(&cfg.endpoint)
        .credentials_provider(creds)
        .force_path_style(true)
        .build();
    S3Client::from_conf(conf)
}

/// 上传对象到 MinIO，返回可访问 URL（domain 优先，否则用 endpoint）
async fn upload_to_minio(
    cfg: &FileMinioConfig,
    object_key: &str,
    data: Vec<u8>,
    content_type: Option<String>,
) -> AppResult<String> {
    let client = build_minio_client(cfg);
    ensure_bucket(&client, &cfg.bucket).await?;

    let mut req = client
        .put_object()
        .bucket(&cfg.bucket)
        .key(object_key)
        .body(ByteStream::from(data));
    if let Some(ct) = content_type {
        req = req.content_type(ct);
    }
    req.send()
        .await
        .map_err(|e| AppError::Business(ResultCode::UploadFileException, Some(e.to_string())))?;

    let base = if cfg.domain.trim().is_empty() {
        cfg.endpoint.trim_end_matches('/')
    } else {
        cfg.domain.trim_end_matches('/')
    };
    Ok(format!("{base}/{}/{object_key}", cfg.bucket))
}

/// 存储桶不存在则创建，并赋予公共读策略（使返回的直链可匿名访问）
async fn ensure_bucket(client: &S3Client, bucket: &str) -> AppResult<()> {
    if client.head_bucket().bucket(bucket).send().await.is_ok() {
        return Ok(());
    }
    client
        .create_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| {
            AppError::Business(
                ResultCode::UploadFileException,
                Some(format!("创建存储桶失败: {e}")),
            )
        })?;
    let policy = format!(
        r#"{{"Version":"2012-10-17","Statement":[{{"Effect":"Allow","Principal":{{"AWS":["*"]}},"Action":["s3:GetObject"],"Resource":["arn:aws:s3:::{bucket}/*"]}}]}}"#
    );
    let _ = client
        .put_bucket_policy()
        .bucket(bucket)
        .policy(policy)
        .send()
        .await;
    Ok(())
}

/// 从上传返回的 URL 反解出对象 key（去掉 base 与 bucket 前缀）
fn extract_minio_key(cfg: &FileMinioConfig, file_path: &str) -> String {
    let base = if cfg.domain.trim().is_empty() {
        cfg.endpoint.trim_end_matches('/')
    } else {
        cfg.domain.trim_end_matches('/')
    };
    let full_prefix = format!("{base}/{}/", cfg.bucket);
    if let Some(key) = file_path.strip_prefix(&full_prefix) {
        return key.to_string();
    }
    let bucket_seg = format!("/{}/", cfg.bucket);
    if let Some(idx) = file_path.find(&bucket_seg) {
        return file_path[idx + bucket_seg.len()..].to_string();
    }
    file_path.trim_start_matches('/').to_string()
}
