#![allow(dead_code)]
//! 全局常量：角色前缀、Redis key 命名空间、通用开关值。

/// 角色权限前缀 ROLE_，鉴权时去除前缀后匹配权限码（如 ROLE_ADMIN → ADMIN）
pub const ROLE_PREFIX: &str = "ROLE_";
pub const ROOT_ROLE_CODE: &str = "ROOT";
pub const MAX_PAGE_SIZE: i64 = 100;

/// Token 类型标识
pub mod token_type {
    pub const ACCESS: &str = "access";
    pub const REFRESH: &str = "refresh";
}

/// Redis Key 前缀
pub mod redis_key {
    pub const CAPTCHA: &str = "captcha:image";
    pub const AUTH_TOKEN: &str = "auth:token";
    pub const JWT_VERSION: &str = "auth:jwt:version";
    pub const USER_SESSIONS: &str = "auth:user:sessions";
    /// 登出后的访问令牌黑名单前缀，完整 Key 为 `auth:token:blacklist:{token}`
    pub const TOKEN_BLACKLIST: &str = "auth:token:blacklist";
    pub const SYS_CONFIG: &str = "sys:config";
    /// 扫码登录票据 Key 前缀，完整 Key 为 `auth:qr_code:{ticket}`
    pub const QR_CODE_PREFIX: &str = "auth:qr_code";
    /// 小程序会话密钥前缀，完整 Key 为 `auth:wxma:session_key:{provider}:{openid}`
    pub const WXMA_SESSION_KEY: &str = "auth:wxma:session_key";
}

/// 微信小程序的认证源标识
pub const WECHAT_MINI_PROVIDER: &str = "wechat-mini-main";

/// 手机/邮箱验证码 Redis Key 前缀（如 captcha:mobile:138...）
pub const MOBILE_CODE: &str = "captcha:mobile";
pub const EMAIL_CODE: &str = "captcha:email";
/// 验证码有效期（秒）
pub const CODE_TTL: u64 = 300;

/// 验证码配置：4 位字符，有效期 300 秒
pub const CAPTCHA_LENGTH: usize = 4;
pub const CAPTCHA_TTL: u64 = 300;
/// 新建用户 / 重置密码时使用的默认密码
pub const DEFAULT_PASSWORD: &str = "123456";
