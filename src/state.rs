//! 应用全局共享状态：DB 连接池、Redis 连接、JWT 令牌管理器、消息推送中枢。
//!
//! `AppState` 通过 `axum::extract::State` 注入到每个 handler。

use crate::config::Settings;
use crate::modules::auth::token::DynTokenManager;
use crate::modules::realtime::hub::MessageHub;
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 全局共享状态 — 注入到 axum Router
#[derive(Clone)]
pub struct AppState {
    /// 数据库连接
    pub db: DatabaseConnection,
    /// Redis 连接
    pub redis: ConnectionManager,
    /// Token 管理器（JWT / Redis-Token 双模式）
    pub token_manager: Arc<DynTokenManager>,
    /// 实时连接管理器
    pub message_hub: Arc<MessageHub>,
    /// 应用配置
    pub settings: Arc<Settings>,
}
