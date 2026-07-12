//! 代码生成器路由与接口
//!
//! 提供表列表、生成配置、代码预览与 zip 下载等代码生成接口。

pub mod service;
pub mod templates;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::core::state::AppState;
use crate::framework::web::error::{AppError, AppResult};
use crate::framework::web::response::Result;
use crate::modules::codegen::service::{CodegenService, ColumnInfo};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableVO {
    pub table_name: String,
    pub table_comment: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnVO {
    pub column_name: String,
    pub data_type: String,
    pub column_comment: String,
    pub is_nullable: String,
    pub column_key: String,
    pub is_pk: bool,
    pub rust_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableListVO {
    pub list: Vec<TableVO>,
    pub total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewVO {
    pub path: String,
    pub file_name: String,
    pub content: String,
    pub scope: String,
    pub language: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenConfigVO {
    pub table_name: String,
    pub entity_name: String,
    pub module_name: String,
    pub business_name: String,
    pub package_name: String,
    pub author: String,
    pub columns: Vec<ColumnVO>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/codegen/table", get(list_tables))
        .route("/api/v1/codegen/{table_name}/config", get(get_config))
        .route("/api/v1/codegen/{table_name}/preview", get(preview))
        .route("/api/v1/codegen/{table_name}/download", get(download))
}

/// 获取数据表列表（来自 PostgreSQL information_schema）
async fn list_tables(State(state): State<AppState>) -> AppResult<Json<Result<TableListVO>>> {
    let tables = CodegenService::new(&state.db).list_tables().await?;
    let list = tables
        .into_iter()
        .map(|(table_name, table_comment)| TableVO { table_name, table_comment })
        .collect::<Vec<_>>();
    let total = list.len() as u64;
    Ok(Json(Result::success(TableListVO { list, total })))
}

/// 获取代码生成配置（由元数据推导）
async fn get_config(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
) -> AppResult<Json<Result<GenConfigVO>>> {
    let meta = CodegenService::new(&state.db).config(&table_name).await?;
    let columns = meta
        .columns
        .iter()
        .map(|c: &ColumnInfo| ColumnVO {
            column_name: c.name.clone(),
            data_type: c.rust_type.clone(),
            column_comment: c.comment.clone(),
            is_nullable: if c.is_nullable { "YES".into() } else { "NO".into() },
            column_key: if c.is_pk { "PRI".into() } else { String::new() },
            is_pk: c.is_pk,
            rust_type: c.rust_type.clone(),
        })
        .collect();
    let config = GenConfigVO {
        table_name: meta.table_name.clone(),
        entity_name: meta.entity_name.clone(),
        module_name: meta.module_name.clone(),
        business_name: meta.entity_lower.clone(),
        package_name: "youlai.rust".into(),
        author: "youlai-rust".into(),
        columns,
    };
    Ok(Json(Result::success(config)))
}

/// 预览生成的代码
async fn preview(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
) -> AppResult<Json<Result<Vec<PreviewVO>>>> {
    let files = CodegenService::new(&state.db).preview(&table_name).await?;
    let list = files
        .into_iter()
        .map(|f| PreviewVO {
            path: f.path,
            file_name: f.file_name,
            content: f.content,
            scope: "backend".into(),
            language: "rs".into(),
        })
        .collect();
    Ok(Json(Result::success(list)))
}

/// 下载生成的代码（zip）
async fn download(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
) -> AppResult<axum::response::Response> {
    let bytes = CodegenService::new(&state.db).download(&table_name).await?;
    let resp = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/zip")
        .header("Content-Disposition", "attachment; filename=\"codegen.zip\"")
        .body(axum::body::Body::from(bytes))
        .map_err(|e| AppError::Business(crate::common::enums::ResultCode::SystemError, Some(e.to_string())))?;
    Ok(resp)
}
