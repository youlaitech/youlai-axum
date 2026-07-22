//! axum 请求提取器：从 Extensions 中取出 `Principal`，无需鉴权的接口可省略。
//!
//! `AuthUser` 实现 `FromRequestParts`，缺失时直接返回 401。

use crate::modules::auth::principal::Principal;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, Extensions, StatusCode},
};

/// 从请求扩展中提取当前登录用户
pub fn current_user(ext: &Extensions) -> Option<&Principal> {
    ext.get::<Principal>()
}

/// 可在 handler 中直接注入当前用户的 extractor
pub struct AuthUser(pub Principal);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Principal>()
            .cloned()
            .map(AuthUser)
            .ok_or((StatusCode::UNAUTHORIZED, "unauthorized"))
    }
}
