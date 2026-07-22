//! 系统管理模块：用户 / 角色 / 菜单 / 部门 / 字典 / 配置 / 通知 / 日志 CRUD。
//!
//! 路由入口 `/api/v1/users`, `/api/v1/roles`, `/api/v1/menus`, `/api/v1/depts` 等。

pub mod config;
pub mod data_scope;
pub mod dept;
pub mod dict;
pub mod log;
pub mod menu;
pub mod notice;
pub mod role;
pub mod user;

use crate::state::AppState;
use axum::Router;

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
