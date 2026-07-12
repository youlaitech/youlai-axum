use validator::Validate;
pub mod dto;
pub mod service;
pub mod wxma;

use axum::{extract::State, routing::{get, post, delete}, Json, Router};
use crate::core::state::AppState;
use crate::framework::web::error::AppResult;
use crate::framework::web::response::Result;
use crate::modules::auth::dto::*;
use crate::modules::auth::service::AuthService;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/captcha", get(get_captcha))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/login/sms", post(login_by_sms))
        .route("/api/v1/auth/sms/code", post(send_sms_code))
        .route("/api/v1/auth/logout", delete(logout))
        .route("/api/v1/auth/refresh-token", post(refresh_token))
        .merge(wxma::routes())
}

/// 获取图形验证码
async fn get_captcha(State(state): State<AppState>) -> AppResult<Json<Result<CaptchaResp>>> {
    let captcha = crate::framework::captcha::service::generate(&state.redis).await?;
    Ok(Json(Result::success(CaptchaResp {
        captcha_id: captcha.captcha_id,
        captcha_base64: captcha.captcha_base64,
    })))
}

/// 用户名密码登录
async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> AppResult<Json<Result<LoginResp>>> {
    req.validate()?;
    let result = AuthService::new(&state.db, &state.redis, &state.token_manager).login(&req).await?;
    Ok(Json(Result::success(result)))
}

async fn login_by_sms(State(state): State<AppState>, Json(req): Json<SmsLoginRequest>) -> AppResult<Json<Result<LoginResp>>> {
    req.validate()?;
    let result = AuthService::new(&state.db, &state.redis, &state.token_manager).login_by_sms(&req).await?;
    Ok(Json(Result::success(result)))
}

async fn send_sms_code(State(state): State<AppState>, Json(req): Json<SendSmsRequest>) -> AppResult<Json<Result<()>>> {
    AuthService::new(&state.db, &state.redis, &state.token_manager).send_sms_code(&req).await?;
    Ok(Json(Result::success_empty()))
}

/// 用户登出
async fn logout() -> AppResult<Json<Result<()>>> {
    Ok(Json(Result::success_empty()))
}

/// 刷新访问令牌
async fn refresh_token(State(state): State<AppState>, Json(req): Json<RefreshTokenRequest>) -> AppResult<Json<Result<LoginResp>>> {
    let result = AuthService::new(&state.db, &state.redis, &state.token_manager).refresh_token(&req).await?;
    Ok(Json(Result::success(result)))
}
