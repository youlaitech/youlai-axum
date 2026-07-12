use axum::{
    extract::FromRequestParts,
    http::{request::Parts, Extensions, StatusCode},
};
use crate::framework::security::model::SecurityUserDetails;

/// 从请求扩展中提取当前登录用户
pub fn current_user(ext: &Extensions) -> Option<&SecurityUserDetails> {
    ext.get::<SecurityUserDetails>()
}

/// 可在 handler 中直接注入当前用户的 extractor
pub struct AuthUser(pub SecurityUserDetails);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<SecurityUserDetails>()
            .cloned()
            .map(AuthUser)
            .ok_or((StatusCode::UNAUTHORIZED, "unauthorized"))
    }
}
