use serde::{Deserialize, Serialize};

/// 角色数据权限
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDataScope {
    pub data_scope: i32,
    pub dept_ids: Option<Vec<i64>>,
}

/// 用户认证详情
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityUserDetails {
    pub user_id: i64,
    pub username: String,
    pub password: String,
    pub enabled: bool,
    pub dept_id: Option<i64>,
    pub data_scopes: Vec<RoleDataScope>,
    pub roles: Vec<String>,
    pub perms: Vec<String>,
    pub is_root: bool,
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
