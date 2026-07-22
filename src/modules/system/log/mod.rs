pub mod dto;
pub mod service;

use crate::error::AppResult;
use crate::modules::auth::permission::require_perm;
use crate::modules::system::log::dto::*;
use crate::modules::system::log::service::LogService;
use crate::response::{PageResult, Result};
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    middleware,
    routing::get,
    Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/logs",
            get(list_logs).route_layer(middleware::from_fn(require_perm("sys:log:list"))),
        )
        .route("/api/v1/logs/analytics/trend", get(get_visit_trend))
        .route("/api/v1/logs/analytics/overview", get(get_visit_overview))
}

async fn list_logs(
    State(state): State<AppState>,
    Query(q): Query<LogQuery>,
) -> AppResult<Json<PageResult<LogVO>>> {
    let p = LogService::new(&state.db).get_page(&q).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

async fn get_visit_trend(
    State(state): State<AppState>,
) -> AppResult<Json<Result<serde_json::Value>>> {
    let data = LogService::new(&state.db).get_visit_trend().await?;
    Ok(Json(Result::success(data)))
}

/// 获取访问概览统计
async fn get_visit_overview(
    State(state): State<AppState>,
) -> AppResult<Json<Result<serde_json::Value>>> {
    let data = LogService::new(&state.db).get_visit_overview().await?;
    Ok(Json(Result::success(data)))
}
