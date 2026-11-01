//! 微信小程序认证：静默登录 / 手机号快捷登录 / 绑定手机号
//!
//! 通过微信开放接口换取 openid 与手机号，
//! 查找或创建用户并签发访问令牌。
//!
//! 设计要点：网络 I/O（`reqwest` 调微信服务器）与纯逻辑（解析 JSON 响应）分离，
//! 解析函数抽到模块级纯函数，可以不连外网直接单测验证解析正确性。

use axum::{
    extract::{Query, State},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use validator::Validate;

use crate::config::WxMaConfig;
use crate::error::{AppError, AppResult};
use crate::modules::auth::dto::*;
use crate::modules::auth::service::AuthService;
use crate::response::Result;
use crate::result_code::ResultCode;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/wxma/silent-login", post(silent_login))
        .route("/api/v1/auth/wxma/phone-login", post(phone_login))
        .route("/api/v1/auth/wxma/bind-mobile", post(bind_mobile))
}

#[derive(Debug, Deserialize)]
pub struct SilentLoginQuery {
    /// 微信登录凭证（wx.login 获取）
    pub code: String,
}

/// 静默登录：通过微信 code 登录，已绑定用户直接返回 token，未绑定返回 openid 需绑定手机号
async fn silent_login(
    State(state): State<AppState>,
    Query(q): Query<SilentLoginQuery>,
) -> AppResult<Json<Result<WxMaLoginVO>>> {
    let cfg = &state.settings.wx.miniapp;
    ensure_wx_configured(cfg)?;
    let session = jscode2session(cfg, &q.code).await?;
    let auth = AuthService::new(&state.db, &state.redis, &state.token_manager);

    // 已绑定 openid 且手机号非空 → 直接签发令牌
    if let Some(user) = auth.find_user_by_openid(&session.openid).await? {
        if let Some(mobile) = user.mobile.as_ref().filter(|m| !m.is_empty()) {
            let token = auth.login_by_mobile(mobile).await?;
            return Ok(Json(Result::success(WxMaLoginVO {
                is_new_user: false,
                need_bind_mobile: false,
                openid: Some(session.openid),
                access_token: Some(token.access_token),
                refresh_token: Some(token.refresh_token),
                token_type: Some(token.token_type),
                expires_in: Some(token.expires_in),
            })));
        }
    }
    // 未绑定或无手机号 → 返回 openid，前端引导绑定手机号
    Ok(Json(Result::success(WxMaLoginVO {
        is_new_user: true,
        need_bind_mobile: true,
        openid: Some(session.openid),
        access_token: None,
        refresh_token: None,
        token_type: None,
        expires_in: None,
    })))
}

/// 手机号快捷登录：同时使用微信 code 和手机号授权 code，一步完成登录（自动创建新用户）
async fn phone_login(
    State(state): State<AppState>,
    Json(req): Json<WxMaPhoneLoginForm>,
) -> AppResult<Json<Result<LoginResp>>> {
    req.validate()?;
    let cfg = &state.settings.wx.miniapp;
    ensure_wx_configured(cfg)?;
    let session = jscode2session(cfg, &req.login_code).await?;
    let mobile = get_phone_number(cfg, &req.phone_code).await?;
    let auth = AuthService::new(&state.db, &state.redis, &state.token_manager);
    let user = auth.find_or_create_user_by_mobile(&mobile).await?;
    auth.bind_wechat_openid(user.id, &session.openid, Some(&session.session_key))
        .await?;
    let token = auth.login_by_mobile(&mobile).await?;
    Ok(Json(Result::success(token)))
}

/// 绑定手机号：校验短信验证码后绑定 openid 并自动登录
async fn bind_mobile(
    State(state): State<AppState>,
    Json(req): Json<WxMaBindMobileForm>,
) -> AppResult<Json<Result<LoginResp>>> {
    req.validate()?;
    // 校验短信验证码（与 /api/v1/auth/sms/code 发送的 key 一致）
    let key = format!("sms:{}", &req.mobile);
    let mut r = state.redis.clone();
    let stored: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut r)
        .await
        .ok()
        .flatten();
    if stored.as_deref() != Some(req.sms_code.as_str()) {
        return Err(AppError::Business(
            ResultCode::UserVerificationCodeError,
            None,
        ));
    }
    let _: () = redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut r)
        .await
        .unwrap_or(());

    let auth = AuthService::new(&state.db, &state.redis, &state.token_manager);
    let user = auth.find_or_create_user_by_mobile(&req.mobile).await?;
    auth.bind_wechat_openid(user.id, &req.openid, None)
        .await?;
    let token = auth.login_by_mobile(&req.mobile).await?;
    Ok(Json(Result::success(token)))
}

/// 校验微信小程序配置是否就绪
fn ensure_wx_configured(cfg: &WxMaConfig) -> AppResult<()> {
    if cfg.appid.is_empty() || cfg.secret.is_empty() {
        return Err(AppError::Business(
            ResultCode::SystemError,
            Some("微信小程序未配置 appid/secret".into()),
        ));
    }
    Ok(())
}

// ==================== 微信开放接口调用 ====================

/// `sns/jscode2session` 响应
#[derive(Debug, Deserialize)]
struct Jscode2SessionResp {
    #[serde(default)]
    openid: String,
    #[serde(default)]
    session_key: String,
    errcode: Option<i32>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResp {
    #[serde(default)]
    access_token: String,
    errcode: Option<i32>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PhoneNumberResp {
    errcode: Option<i32>,
    errmsg: Option<String>,
    phone_info: Option<PhoneInfo>,
}

/// 微信手机号信息为驼峰命名（phoneNumber / purePhoneNumber）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhoneInfo {
    phone_number: Option<String>,
    pure_phone_number: Option<String>,
}

/// 调用 `sns/jscode2session` 换取 openid 与 session_key
async fn jscode2session(cfg: &WxMaConfig, code: &str) -> AppResult<Jscode2SessionResp> {
    let url = format!(
        "https://api.weixin.qq.com/sns/jscode2session?appid={}&secret={}&js_code={}&grant_type=authorization_code",
        cfg.appid, cfg.secret, code
    );
    let text = reqwest::get(&url)
        .await
        .map_err(|e| {
            AppError::Business(ResultCode::SystemError, Some(format!("微信登录失败：{e}")))
        })?
        .text()
        .await
        .map_err(|e| {
            AppError::Business(ResultCode::SystemError, Some(format!("微信登录失败：{e}")))
        })?;
    parse_jscode2session(&text)
}

/// 调用 `getuserphonenumber` 换取手机号（需先获取 access_token）
async fn get_phone_number(cfg: &WxMaConfig, phone_code: &str) -> AppResult<String> {
    let token_url = format!(
        "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid={}&secret={}",
        cfg.appid, cfg.secret
    );
    let token_text = reqwest::get(&token_url)
        .await
        .map_err(|e| {
            AppError::Business(
                ResultCode::SystemError,
                Some(format!("获取 access_token 失败：{e}")),
            )
        })?
        .text()
        .await
        .map_err(|e| {
            AppError::Business(
                ResultCode::SystemError,
                Some(format!("获取 access_token 失败：{e}")),
            )
        })?;
    let access_token = parse_access_token(&token_text)?;

    let phone_url = format!(
        "https://api.weixin.qq.com/wxa/business/getuserphonenumber?access_token={}",
        access_token
    );
    let phone_text = reqwest::Client::new()
        .post(&phone_url)
        .json(&serde_json::json!({ "code": phone_code }))
        .send()
        .await
        .map_err(|e| {
            AppError::Business(
                ResultCode::SystemError,
                Some(format!("获取手机号失败：{e}")),
            )
        })?
        .text()
        .await
        .map_err(|e| {
            AppError::Business(
                ResultCode::SystemError,
                Some(format!("获取手机号失败：{e}")),
            )
        })?;
    parse_phone_number_resp(&phone_text)
}

/// 纯函数：解析 `jscode2session` 响应（离线可测）
fn parse_jscode2session(body: &str) -> AppResult<Jscode2SessionResp> {
    let resp: Jscode2SessionResp = serde_json::from_str(body).map_err(|e| {
        AppError::Business(
            ResultCode::SystemError,
            Some(format!("微信响应解析失败：{e}")),
        )
    })?;
    if resp.errcode.unwrap_or(0) != 0 {
        return Err(AppError::Business(
            ResultCode::SystemError,
            Some(format!("微信登录失败：{}", resp.errmsg.unwrap_or_default())),
        ));
    }
    if resp.openid.is_empty() || resp.session_key.is_empty() {
        return Err(AppError::Business(
            ResultCode::SystemError,
            Some("微信登录失败：响应缺少 openid/session_key".into()),
        ));
    }
    Ok(resp)
}

/// 纯函数：解析 `token` 接口响应，提取 access_token（离线可测）
fn parse_access_token(body: &str) -> AppResult<String> {
    let resp: AccessTokenResp = serde_json::from_str(body).map_err(|e| {
        AppError::Business(
            ResultCode::SystemError,
            Some(format!("微信响应解析失败：{e}")),
        )
    })?;
    if resp.errcode.unwrap_or(0) != 0 {
        return Err(AppError::Business(
            ResultCode::SystemError,
            Some(format!(
                "获取 access_token 失败：{}",
                resp.errmsg.unwrap_or_default()
            )),
        ));
    }
    Ok(resp.access_token)
}

/// 纯函数：解析 `getuserphonenumber` 响应，提取手机号（离线可测）
fn parse_phone_number_resp(body: &str) -> AppResult<String> {
    let resp: PhoneNumberResp = serde_json::from_str(body).map_err(|e| {
        AppError::Business(
            ResultCode::SystemError,
            Some(format!("微信响应解析失败：{e}")),
        )
    })?;
    if resp.errcode.unwrap_or(0) != 0 {
        return Err(AppError::Business(
            ResultCode::SystemError,
            Some(format!(
                "获取手机号失败：{}",
                resp.errmsg.unwrap_or_default()
            )),
        ));
    }
    resp.phone_info
        .and_then(|p| p.phone_number.or(p.pure_phone_number))
        .ok_or_else(|| AppError::Business(ResultCode::SystemError, Some("获取手机号失败".into())))
}

// ==================== 离线单测 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_jscode2session_success() {
        let body = r#"{"openid":"oABC123","session_key":"sk_xyz","unionid":"u_1"}"#;
        let r = parse_jscode2session(body).unwrap();
        assert_eq!(r.openid, "oABC123");
        assert_eq!(r.session_key, "sk_xyz");
        assert_eq!(r.unionid.as_deref(), Some("u_1"));
    }

    #[test]
    fn parse_jscode2session_errcode_returns_error() {
        let body = r#"{"errcode":40029,"errmsg":"invalid code"}"#;
        let err = parse_jscode2session(body).unwrap_err();
        // 错误信息透传微信的 errmsg，方便排查
        assert!(format!("{:?}", err).contains("invalid code"));
    }

    #[test]
    fn parse_access_token_success() {
        let body = r#"{"access_token":"TOKEN_ABC","expires_in":7200}"#;
        assert_eq!(parse_access_token(body).unwrap(), "TOKEN_ABC");
    }

    #[test]
    fn parse_access_token_errcode_returns_error() {
        let body = r#"{"errcode":40125,"errmsg":"invalid appsecret"}"#;
        assert!(parse_access_token(body).is_err());
    }

    #[test]
    fn parse_phone_number_prefers_phone_number() {
        let body =
            r#"{"phone_info":{"phoneNumber":"13800000000","purePhoneNumber":"13800000000"}}"#;
        assert_eq!(parse_phone_number_resp(body).unwrap(), "13800000000");
    }

    #[test]
    fn parse_phone_number_falls_back_to_pure() {
        let body = r#"{"phone_info":{"purePhoneNumber":"13900000000"}}"#;
        assert_eq!(parse_phone_number_resp(body).unwrap(), "13900000000");
    }

    #[test]
    fn parse_phone_number_missing_returns_error() {
        let body = r#"{"phone_info":{}}"#;
        assert!(parse_phone_number_resp(body).is_err());
    }

    #[test]
    fn parse_phone_number_errcode_returns_error() {
        let body = r#"{"errcode":-1,"errmsg":"busy"}"#;
        assert!(parse_phone_number_resp(body).is_err());
    }
}
