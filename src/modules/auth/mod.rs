//! 认证模块：登录/登出、验证码、JWT 令牌签发与校验、权限拦截、扫码登录。
//!
//! 路由入口 `/api/v1/auth/*`，含扫码登录 `/api/v1/auth/qr-code/*`。

use validator::Validate;
pub mod captcha;
pub mod dto;
pub mod extractor;
pub mod middleware;
pub mod permission;
pub mod principal;
pub mod qrcode;
pub mod service;
pub mod token;
pub mod wxma;

use crate::error::AppResult;
use crate::modules::auth::dto::*;
use crate::modules::auth::service::AuthService;
use crate::response::Result;
use crate::state::AppState;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/captcha", get(get_captcha))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/login/sms", post(login_by_sms))
        .route("/api/v1/auth/sms/code", post(send_sms_code))
        .route("/api/v1/auth/logout", delete(logout))
        .route("/api/v1/auth/refresh-token", post(refresh_token))
        .merge(wxma::routes())
        .merge(qrcode::routes())
}

/// 获取图形验证码
async fn get_captcha(State(state): State<AppState>) -> AppResult<Json<Result<CaptchaResp>>> {
    let captcha = crate::modules::auth::captcha::generate(&state.redis).await?;
    Ok(Json(Result::success(CaptchaResp {
        captcha_id: captcha.captcha_id,
        captcha_base64: captcha.captcha_base64,
    })))
}

/// 用户名密码登录
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<Result<LoginResp>>> {
    req.validate()?;
    // 按账号限流（5 次/60 秒），防爆破同时避免同 IP 多账号互挤
    if !crate::rate_limit::login_rate_allowed(&state.redis, &req.username).await? {
        return Err(crate::rate_limit::login_rate_limited());
    }
    let result = AuthService::new(&state.db, &state.redis, &state.token_manager)
        .login(&req)
        .await?;
    Ok(Json(Result::success(result)))
}

async fn login_by_sms(
    State(state): State<AppState>,
    Json(req): Json<SmsLoginRequest>,
) -> AppResult<Json<Result<LoginResp>>> {
    req.validate()?;
    let result = AuthService::new(&state.db, &state.redis, &state.token_manager)
        .login_by_sms(&req)
        .await?;
    Ok(Json(Result::success(result)))
}

async fn send_sms_code(
    State(state): State<AppState>,
    Json(req): Json<SendSmsRequest>,
) -> AppResult<Json<Result<()>>> {
    AuthService::new(&state.db, &state.redis, &state.token_manager)
        .send_sms_code(&req)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 用户登出
async fn logout() -> AppResult<Json<Result<()>>> {
    Ok(Json(Result::success_empty()))
}

/// 刷新访问令牌
async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> AppResult<Json<Result<LoginResp>>> {
    let result = AuthService::new(&state.db, &state.redis, &state.token_manager)
        .refresh_token(&req)
        .await?;
    Ok(Json(Result::success(result)))
}
