use axum::{extract::State, routing::get, Json, Router};
use crate::core::state::AppState;
use crate::framework::sse;
use crate::framework::web::response::Result;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/sse/connect", get(sse::sse_connect))
        .route("/api/v1/sse/online-count", get(online_count))
}

/// 获取 SSE 在线用户数
async fn online_count(State(state): State<AppState>) -> Json<Result<i64>> {
    Json(Result::success(state.sse.online_count() as i64))
}
