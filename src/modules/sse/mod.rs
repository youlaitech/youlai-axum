//! SSE 实时推送模块：在线人数广播、字典变更通知、公告发布/撤回。

pub mod handler;
pub mod hub;
pub mod topics;

pub use handler::{online_count, sse_connect};

use crate::state::AppState;
use axum::{routing::get, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/sse/connect", get(sse_connect))
        .route("/api/v1/sse/online-count", get(online_count))
}
