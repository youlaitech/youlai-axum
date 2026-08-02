//! 文件上传/删除模块：支持 MinIO（S3 兼容）与本地磁盘两种后端。

pub mod dto;
pub mod handler;
pub mod service;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

use self::handler::{delete_file, upload_file};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/files", post(upload_file).delete(delete_file))
}
