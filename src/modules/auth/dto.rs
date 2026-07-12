use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 64))]
    pub username: String,
    #[validate(length(min = 6, max = 32))]
    pub password: String,
    pub captcha_id: Option<String>,
    pub captcha_code: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SmsLoginRequest {
    #[validate(length(min = 1))]
    pub mobile: String,
    #[validate(length(min = 1))]
    pub code: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendSmsRequest {
    #[validate(length(min = 1))]
    pub mobile: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptchaResp {
    pub captcha_id: String,
    pub captcha_base64: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResp {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresIn")]
    pub expires_in: i64,
    #[serde(rename = "tokenType")]
    pub token_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UserInfoVO {
    pub user_id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub roles: Vec<String>,
    pub perms: Vec<String>,
}

/// 微信小程序手机号快捷登录请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WxMaPhoneLoginForm {
    /// 微信登录凭证（wx.login 获取）
    #[validate(length(min = 1))]
    pub login_code: String,
    /// 手机号授权凭证（getPhoneNumber 事件获取）
    #[validate(length(min = 1))]
    pub phone_code: String,
}

/// 微信小程序绑定手机号请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WxMaBindMobileForm {
    /// 微信用户唯一标识
    #[validate(length(min = 1))]
    pub openid: String,
    /// 手机号码
    #[validate(length(min = 1))]
    pub mobile: String,
    /// 短信验证码
    #[validate(length(min = 1))]
    pub sms_code: String,
}

/// 微信小程序登录响应（静默登录返回）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WxMaLoginVO {
    /// 是否新用户
    pub is_new_user: bool,
    /// 是否需要绑定手机号
    pub need_bind_mobile: bool,
    /// 微信 openid
    pub openid: Option<String>,
    /// 访问令牌（已绑定用户返回）
    pub access_token: Option<String>,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 令牌类型
    pub token_type: Option<String>,
    /// 过期时间（秒）
    pub expires_in: Option<i64>,
}
