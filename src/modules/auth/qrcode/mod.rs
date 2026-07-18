//! 扫码登录接口：generate/status/login 不需要登录态（PC 端未登录），由全局鉴权中间件放行；
//! scan/confirm/cancel 需要 APP 端登录态，当前用户 ID 从请求扩展中的 Principal 获取。

pub mod model;
pub mod service;
pub mod util;

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use validator::Validate;

use crate::error::AppResult;
use crate::modules::auth::dto::LoginResp;
use crate::modules::auth::extractor::AuthUser;
use crate::modules::auth::qrcode::model::*;
use crate::modules::auth::qrcode::service::QrCodeLoginService;
use crate::response::Result;
use crate::state::AppState;

/// 扫码登录路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/qr-code/generate", post(generate))
        .route("/api/v1/auth/qr-code/status", get(status))
        .route("/api/v1/auth/qr-code/scan", post(scan))
        .route("/api/v1/auth/qr-code/confirm", post(confirm))
        .route("/api/v1/auth/qr-code/cancel", post(cancel))
        .route("/api/v1/auth/qr-code/login", post(login))
}

/// 生成扫码登录票据
async fn generate(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Result<QrCodeGenerateVO>>> {
    let client_ip = extract_client_ip(&headers);
    let svc = QrCodeLoginService::new(&state.db, &state.redis, &state.token_manager);
    let resp = svc.generate(&client_ip).await?;
    Ok(Json(Result::success(resp)))
}

/// 查询扫码状态
async fn status(
    State(state): State<AppState>,
    Query(query): Query<QrCodeStatusQuery>,
) -> AppResult<Json<Result<QrCodeStatusVO>>> {
    let svc = QrCodeLoginService::new(&state.db, &state.redis, &state.token_manager);
    let resp = svc.status(&query.ticket).await?;
    Ok(Json(Result::success(resp)))
}

/// APP 标记已扫码
async fn scan(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Json(req): Json<QrCodeTicketForm>,
) -> AppResult<Json<Result<QrCodeStatusVO>>> {
    req.validate()?;
    let svc = QrCodeLoginService::new(&state.db, &state.redis, &state.token_manager);
    let resp = svc.scan(&req.ticket, principal.user_id).await?;
    Ok(Json(Result::success(resp)))
}

/// APP 确认登录
async fn confirm(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Json(req): Json<QrCodeTicketForm>,
) -> AppResult<Json<Result<QrCodeStatusVO>>> {
    req.validate()?;
    let svc = QrCodeLoginService::new(&state.db, &state.redis, &state.token_manager);
    let resp = svc.confirm(&req.ticket, principal.user_id).await?;
    Ok(Json(Result::success(resp)))
}

/// APP 取消登录
async fn cancel(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Json(req): Json<QrCodeTicketForm>,
) -> AppResult<Json<Result<QrCodeStatusVO>>> {
    req.validate()?;
    let svc = QrCodeLoginService::new(&state.db, &state.redis, &state.token_manager);
    let resp = svc.cancel(&req.ticket, principal.user_id).await?;
    Ok(Json(Result::success(resp)))
}

/// PC 端换取会话令牌
async fn login(
    State(state): State<AppState>,
    Json(req): Json<QrCodeTicketForm>,
) -> AppResult<Json<Result<LoginResp>>> {
    req.validate()?;
    let svc = QrCodeLoginService::new(&state.db, &state.redis, &state.token_manager);
    let resp = svc.login(&req.ticket).await?;
    Ok(Json(Result::success(resp)))
}

/// 从请求头或连接信息中提取客户端 IP，兼容反向代理
fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(v) = headers.get("X-Forwarded-For") {
        if let Ok(s) = v.to_str() {
            let s = s.trim();
            if !s.is_empty() && s.to_lowercase() != "unknown" {
                let idx = s.find(',').unwrap_or(s.len());
                return s[..idx].trim().to_string();
            }
        }
    }
    if let Some(v) = headers.get("X-Real-IP") {
        if let Ok(s) = v.to_str() {
            let s = s.trim();
            if !s.is_empty() && s.to_lowercase() != "unknown" {
                return s.to_string();
            }
        }
    }
    "unknown".to_string()
}
