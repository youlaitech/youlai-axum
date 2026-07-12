use std::sync::Arc;
use sea_orm::DatabaseConnection;
use redis::aio::ConnectionManager;
use crate::core::config::Settings;
use crate::framework::security::token::DynTokenManager;
use crate::framework::sse::manager::SseManager;

/// 全局共享状态 — 注入到 axum Router
#[derive(Clone)]
pub struct AppState {
    /// 数据库连接
    pub db: DatabaseConnection,
    /// Redis 连接
    pub redis: ConnectionManager,
    /// Token 管理器（JWT / Redis-Token 双模式）
    pub token_manager: Arc<DynTokenManager>,
    /// SSE 管理器
    pub sse: Arc<SseManager>,
    /// 应用配置
    pub settings: Arc<Settings>,
}
