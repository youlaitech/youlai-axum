use serde::{Deserialize, Serialize};
use validator::Validate;

/// 扫码登录状态：WAITING → SCANNED → CONFIRMED → LOGGED_IN / CANCELED / EXPIRED
pub const STATUS_WAITING: &str = "WAITING";
pub const STATUS_SCANNED: &str = "SCANNED";
pub const STATUS_CONFIRMED: &str = "CONFIRMED";
pub const STATUS_LOGGED_IN: &str = "LOGGED_IN";
pub const STATUS_CANCELED: &str = "CANCELED";
pub const STATUS_EXPIRED: &str = "EXPIRED";

/// 扫码登录票据上下文，序列化为 JSON 存入 Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeLoginContext {
    /// 票据，UUID 无连字符
    pub ticket: String,
    /// 状态枚举名
    pub status: String,
    /// 扫码用户 ID，scan 时写入
    pub user_id: Option<i64>,
    /// 用户原始昵称（未脱敏）
    pub nickname: Option<String>,
    /// 用户头像 URL
    pub avatar: Option<String>,
    /// 创建时间戳（毫秒）
    pub created_at: Option<i64>,
    /// 扫码时间戳（毫秒）
    pub scanned_at: Option<i64>,
    /// 确认时间戳（毫秒）
    pub confirmed_at: Option<i64>,
    /// generate 时的 PC 端 IP，用于审计
    pub client_ip: Option<String>,
}

/// generate 接口响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeGenerateVO {
    pub ticket: String,
    pub expire_seconds: i64,
}

/// status/scan/confirm/cancel 接口响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeStatusVO {
    pub ticket: String,
    pub status: String,
    /// 脱敏昵称，SCANNED 之后返回
    pub nickname: Option<String>,
    /// 头像 URL，SCANNED 之后返回
    pub avatar: Option<String>,
    /// 剩余有效期（秒）
    pub expire_seconds: i64,
}

/// status 接口查询参数
#[derive(Debug, Deserialize)]
pub struct QrCodeStatusQuery {
    pub ticket: String,
}

/// scan/confirm/cancel/login 接口请求体
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeTicketForm {
    #[validate(length(min = 1))]
    pub ticket: String,
}
