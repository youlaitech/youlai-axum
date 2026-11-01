//! 实时推送模块：在线状态、字典变更与公告发布撤回

pub mod handler;
pub mod hub;
pub mod ticket;
pub mod topics;

pub use hub::MessageHub;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/realtime/tickets", post(handler::create_ticket))
        .route("/api/v1/realtime/presence", get(handler::presence))
        .route("/api/v1/realtime/ws", get(handler::ws_connect))
}
