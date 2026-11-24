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
use serde::Deserialize;

/// 访问趋势查询参数
#[derive(Debug, Deserialize)]
pub struct VisitTrendQuery {
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/logs",
            get(list_logs).route_layer(middleware::from_fn(require_perm("sys:log:list"))),
        )
        .route("/api/v1/analytics/visits/summary", get(get_visit_summary))
        .route("/api/v1/analytics/visits/trend", get(get_visit_trend))
        .route("/api/v1/analytics/users/summary", get(get_user_summary))
}

async fn list_logs(
    State(state): State<AppState>,
    Query(q): Query<LogQuery>,
) -> AppResult<Json<PageResult<LogVO>>> {
    let p = LogService::new(&state.db).get_page(&q).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 访问汇总
async fn get_visit_summary(
    State(state): State<AppState>,
) -> AppResult<Json<Result<serde_json::Value>>> {
    let data = LogService::new(&state.db).get_visit_summary().await?;
    Ok(Json(Result::success(data)))
}

/// 访问趋势
async fn get_visit_trend(
    State(state): State<AppState>,
    Query(q): Query<VisitTrendQuery>,
) -> AppResult<Json<Result<serde_json::Value>>> {
    let data = LogService::new(&state.db)
        .get_visit_trend(&q.start_date, &q.end_date)
        .await?;
    Ok(Json(Result::success(data)))
}

/// 用户汇总
async fn get_user_summary(
    State(state): State<AppState>,
) -> AppResult<Json<Result<serde_json::Value>>> {
    let data = LogService::new(&state.db).get_user_summary().await?;
    Ok(Json(Result::success(data)))
}
