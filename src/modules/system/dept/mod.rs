use validator::Validate;
pub mod dto;
pub mod service;

use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    middleware, Json, Router,
};
use crate::core::state::AppState;
use crate::framework::security::permission::require_perm;
use crate::framework::web::error::AppResult;
use crate::framework::web::response::Result;
use crate::modules::system::dept::dto::*;
use crate::modules::system::dept::service::DeptService;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/depts", get(list_depts).route_layer(middleware::from_fn(require_perm("sys:dept:list"))))
        .route("/api/v1/depts/options", get(list_dept_options))
        .route("/api/v1/depts", post(create_dept).route_layer(middleware::from_fn(require_perm("sys:dept:create"))))
        .route("/api/v1/depts/{id}/form", get(get_dept_form).route_layer(middleware::from_fn(require_perm("sys:dept:query"))))
        .route("/api/v1/depts/{id}", put(update_dept).route_layer(middleware::from_fn(require_perm("sys:dept:update"))))
        .route("/api/v1/depts/{id}", delete(delete_depts).route_layer(middleware::from_fn(require_perm("sys:dept:delete"))))
}

/// 查询部门树
async fn list_depts(State(state): State<AppState>) -> AppResult<Json<Result<Vec<DeptVO>>>> {
    Ok(Json(Result::success(DeptService::new(&state.db).get_tree().await?)))
}

async fn get_dept_form(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Result<DeptVO>>> {
    Ok(Json(Result::success(DeptService::new(&state.db).get_form(id).await?)))
}

/// 创建部门
async fn create_dept(State(state): State<AppState>, Json(f): Json<DeptCreate>) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    Ok(Json(Result::success(DeptService::new(&state.db).create(&f).await?)))
}

/// 更新部门
async fn update_dept(State(state): State<AppState>, Path(id): Path<i64>, Json(f): Json<DeptUpdate>) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    let mut f = f;
    f.id = id;
    DeptService::new(&state.db).update(&f).await?;
    Ok(Json(Result::success_empty()))
}

/// 删除部门（支持批量）
async fn delete_depts(State(state): State<AppState>, Path(ids): Path<String>) -> AppResult<Json<Result<u64>>> {
    Ok(Json(Result::success(DeptService::new(&state.db).delete(&ids).await?)))
}

/// 获取部门下拉选项
async fn list_dept_options(State(state): State<AppState>) -> AppResult<Json<Result<Vec<DeptOptionVO>>>> {
    Ok(Json(Result::success(DeptService::new(&state.db).get_options().await?)))
}
