//! 认证主体与角色数据权限。
//!
//! `Principal` 在 JWT 校验通过后构造，包含 userId、用户名、角色列表与权限码集合，
//! 通过 `axum::Extension` 注入到 handler 中。

use serde::{Deserialize, Serialize};

/// 角色数据权限：dataScope + 部门 ID 列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDataScope {
    pub data_scope: i32,
    pub dept_ids: Option<Vec<i64>>,
}

/// 用户认证详情
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Principal {
    pub user_id: i64,
    pub username: String,
    /// 密码哈希只在登录校验时使用，不写入 Redis 会话
    #[serde(default, skip_serializing)]
    pub password: String,
    pub enabled: bool,
    pub dept_id: Option<i64>,
    pub data_scopes: Vec<RoleDataScope>,
    pub roles: Vec<String>,
    pub perms: Vec<String>,
    pub is_root: bool,
    /// 头像 URL
    pub avatar: Option<String>,
}

/// 认证 Token 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationToken {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresIn")]
    pub expires_in: i64,
    #[serde(rename = "tokenType")]
    pub token_type: String,
}

impl AuthenticationToken {
    pub fn new(access_token: String, refresh_token: String, expires_in: i64) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_in,
            token_type: "Bearer".to_string(),
        }
    }
}
