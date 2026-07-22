//! SSE 实时推送模块：在线人数广播、字典变更通知、公告发布/撤回。
//!
//! 路由入口 `/api/v1/sse/*`，基于 tokio broadcast channel 实现服务端主动推送。

pub mod http;
pub mod hub;
pub mod topics;

pub use http::sse_connect;

use crate::response::Result;
use crate::state::AppState;
use axum::{extract::State, routing::get, Json, Router};

/// 组装 SSE 相关路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/sse/connect", get(sse_connect))
        .route("/api/v1/sse/online-count", get(online_count))
}

/// 获取 SSE 在线用户数
async fn online_count(State(state): State<AppState>) -> Json<Result<i64>> {
    Json(Result::success(state.message_hub.online_count() as i64))
}
