pub mod user;
pub mod role;
pub mod menu;
pub mod dept;
pub mod dict;
pub mod config;
pub mod notice;
pub mod log;

use axum::Router;
use crate::core::state::AppState;

/// 组装所有 system 子路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(user::routes())
        .merge(role::routes())
        .merge(menu::routes())
        .merge(dept::routes())
        .merge(dict::routes())
        .merge(config::routes())
        .merge(notice::routes())
        .merge(log::routes())
}
