use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::error::{AppError, AppResult};
use crate::modules::codegen::dto::{
    AiFillConfigForm, GenConfigForm, GenConfigVO, PreviewQuery, PreviewVO, TableListVO,
};
use crate::modules::codegen::service::CodegenService;
use crate::response::Result;
use crate::result_code::ResultCode;
use crate::state::AppState;

/// 获取数据表列表（来自 PostgreSQL information_schema）
pub async fn list_tables(State(state): State<AppState>) -> AppResult<Json<Result<TableListVO>>> {
    let tables = CodegenService::new(&state.db).list_tables().await?;
    let list = tables
        .into_iter()
        .map(|(table_name, table_comment)| crate::modules::codegen::dto::TableVO {
            table_name,
            table_comment,
            engine: "PostgreSQL".into(),
            create_time: None,
        })
        .collect::<Vec<_>>();
    let total = list.len() as u64;
    Ok(Json(Result::success(TableListVO { list, total })))
}

/// 获取代码生成配置
pub async fn get_config(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
) -> AppResult<Json<Result<GenConfigVO>>> {
    let config = CodegenService::new(&state.db)
        .config(&table_name)
        .await?;
    Ok(Json(Result::success(config)))
}

/// AI 填充代码生成配置
pub async fn ai_fill_config(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
    body: Option<Json<AiFillConfigForm>>,
) -> AppResult<Json<Result<GenConfigVO>>> {
    let requirement = body.and_then(|Json(form)| form.requirement);
    let http = reqwest::Client::new();
    let config = CodegenService::new(&state.db)
        .ai_fill_config(&http, &state.settings.ai, &table_name, requirement.as_deref())
        .await?;
    Ok(Json(Result::success(config)))
}

/// 保存代码生成配置
pub async fn save_config(
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
pub async fn delete_config(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
) -> AppResult<Json<Result<()>>> {
    CodegenService::new(&state.db)
        .delete_config(&table_name)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 预览生成的代码
pub async fn preview(
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
pub async fn download(
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
        .map_err(|e| AppError::Business(ResultCode::SystemError, Some(e.to_string())))?;
    Ok(resp)
}
