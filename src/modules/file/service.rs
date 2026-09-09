//! 文件模块的业务逻辑：S3（RustFS）客户端的构建与对象读写。

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;

use crate::config::FileS3Config;
use crate::error::{AppError, AppResult};
use crate::result_code::ResultCode;

/// 构建 S3（RustFS）客户端：路径风格寻址，region 固定占位
fn build_s3_client(cfg: &FileS3Config) -> S3Client {
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

/// 上传对象到 S3（RustFS），返回可访问 URL（domain 优先，否则用 endpoint）
pub async fn upload_to_s3(
    cfg: &FileS3Config,
    object_key: &str,
    data: Vec<u8>,
    content_type: Option<String>,
) -> AppResult<String> {
    let client = build_s3_client(cfg);
    ensure_bucket(&client, &cfg.bucket).await?;

    let mut req = client
        .put_object()
        .bucket(&cfg.bucket)
        .key(object_key)
        .body(ByteStream::from(data));
    if let Some(ct) = content_type {
        req = req.content_type(ct);
    }
    req.send().await.map_err(|e| {
        AppError::Business(ResultCode::UploadFileException, Some(e.to_string()))
    })?;

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
pub fn extract_s3_key(cfg: &FileS3Config, file_path: &str) -> String {
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

/// 删除 S3（RustFS）中指定对象 key
pub async fn delete_s3_object(cfg: &FileS3Config, key: &str) -> AppResult<()> {
    let client = build_s3_client(cfg);
    client
        .delete_object()
        .bucket(&cfg.bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppError::Business(ResultCode::DeleteFileException, Some(e.to_string())))?;
    Ok(())
}
