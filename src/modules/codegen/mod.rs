//! 代码生成器路由与接口
//!
//! 提供表列表、生成配置 CRUD、代码预览与 zip 下载等接口。

pub mod service;
pub mod templates;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::modules::codegen::service::CodegenService;
use crate::response::Result;
use crate::state::AppState;

// ── 查询参数 ──

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    #[serde(default = "default_page_type")]
    pub page_type: String,
    #[serde(rename = "type", default = "default_frontend_type")]
    pub frontend_type: String,
}

fn default_page_type() -> String {
    "classic".into()
}
fn default_frontend_type() -> String {
    "ts".into()
}

// ── 请求体 ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenConfigForm {
    #[serde(default)]
    pub id: Option<i64>,
    pub table_name: String,
    pub business_name: Option<String>,
    pub module_name: Option<String>,
    pub package_name: Option<String>,
    pub entity_name: Option<String>,
    pub author: Option<String>,
    pub parent_menu_id: Option<i64>,
    pub page_type: Option<String>,
    pub remove_table_prefix: Option<String>,
    pub field_configs: Option<Vec<FieldConfigForm>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldConfigForm {
    #[serde(default)]
    pub id: Option<i64>,
    pub column_name: Option<String>,
    pub column_type: Option<String>,
    pub field_name: Option<String>,
    pub field_sort: Option<i32>,
    pub field_type: Option<String>,
    pub field_comment: Option<String>,
    pub is_show_in_list: Option<i16>,
    pub is_show_in_form: Option<i16>,
    pub is_show_in_query: Option<i16>,
    pub is_required: Option<i16>,
    pub max_length: Option<i32>,
    pub form_type: Option<i16>,
    pub query_type: Option<i16>,
    pub dict_type: Option<String>,
}

// ── 响应 VO ──

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableVO {
    pub table_name: String,
    pub table_comment: String,
    pub engine: String,
    pub create_time: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnVO {
    pub column_name: String,
    pub data_type: String,
    pub column_comment: String,
    pub is_nullable: String,
    pub is_pk: bool,
    pub column_key: String,
    pub field_name: String,
    pub field_type: String,
    pub ts_type: String,
    pub is_show_in_list: i16,
    pub is_show_in_form: i16,
    pub is_show_in_query: i16,
    pub is_required: i16,
    pub form_type: Option<i16>,
    pub query_type: Option<i16>,
    pub dict_type: Option<String>,
    pub max_length: Option<i32>,
    pub field_sort: Option<i32>,
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
    pub id: Option<i64>,
    pub table_name: String,
    pub business_name: Option<String>,
    pub module_name: Option<String>,
    pub package_name: Option<String>,
    pub entity_name: String,
    pub author: Option<String>,
    pub parent_menu_id: Option<i64>,
    pub page_type: Option<String>,
    pub remove_table_prefix: Option<String>,
    pub field_configs: Vec<ColumnVO>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/codegen/table", get(list_tables))
        .route(
            "/api/v1/codegen/{table_name}/config",
            get(get_config).post(save_config).delete(delete_config),
        )
        .route("/api/v1/codegen/{table_name}/preview", get(preview))
        .route("/api/v1/codegen/{table_name}/download", get(download))
}

/// 获取数据表列表（来自 PostgreSQL information_schema）
async fn list_tables(State(state): State<AppState>) -> AppResult<Json<Result<TableListVO>>> {
    let tables = CodegenService::new(&state.db).list_tables().await?;
    let list = tables
        .into_iter()
        .map(|(table_name, table_comment)| TableVO {
            table_name,
            table_comment,
            engine: "InnoDB".into(),
            create_time: None,
        })
        .collect::<Vec<_>>();
    let total = list.len() as u64;
    Ok(Json(Result::success(TableListVO { list, total })))
}

/// 获取代码生成配置
async fn get_config(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
) -> AppResult<Json<Result<GenConfigVO>>> {
    let config = CodegenService::new(&state.db)
        .config(&table_name)
        .await?;
    Ok(Json(Result::success(config)))
}

/// 保存代码生成配置
async fn save_config(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
    Json(form): Json<GenConfigForm>,
) -> AppResult<Json<Result<()>>> {
    CodegenService::new(&state.db)
        .save_config(&table_name, &form)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 删除代码生成配置
async fn delete_config(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
) -> AppResult<Json<Result<()>>> {
    CodegenService::new(&state.db)
        .delete_config(&table_name)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 预览生成的代码
async fn preview(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
    Query(q): Query<PreviewQuery>,
) -> AppResult<Json<Result<Vec<PreviewVO>>>> {
    let files = CodegenService::new(&state.db)
        .preview(&table_name, &q.page_type, &q.frontend_type)
        .await?;
    Ok(Json(Result::success(files)))
}

/// 下载生成的代码（zip）
async fn download(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
    Query(q): Query<PreviewQuery>,
) -> AppResult<axum::response::Response> {
    let bytes = CodegenService::new(&state.db)
        .download(&table_name, &q.page_type, &q.frontend_type)
        .await?;
    let resp = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/zip")
        .header(
            "Content-Disposition",
            "attachment; filename=\"youlai-admin-code.zip\"",
        )
        .body(axum::body::Body::from(bytes))
        .map_err(|e| {
            AppError::Business(
                crate::result_code::ResultCode::SystemError,
                Some(e.to_string()),
            )
        })?;
    Ok(resp)
}
