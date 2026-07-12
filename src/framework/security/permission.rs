use axum::{extract::Request, middleware::Next, response::Response, http::StatusCode};
use crate::framework::security::extractor::current_user;

/// 构建指定权限标识的校验中间件。超级管理员自动放行
pub fn require_perm(
    perm: &'static str,
) -> impl Fn(Request, Next) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>,
> + Clone {
    move |req: Request, next: Next| {
        let perm = perm;
        Box::pin(async move {
            let user = match current_user(req.extensions()) {
                Some(u) => u,
                None => return Err(StatusCode::UNAUTHORIZED),
            };
            if user.is_root {
                return Ok(next.run(req).await);
            }
            if user.perms.iter().any(|p| p == perm || p == "*:*:*") {
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        })
    }
}

/// 仅校验登录状态，不检查具体权限
#[allow(dead_code)]
pub fn require_auth() -> impl Fn(Request, Next) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>,
> + Clone {
    |req: Request, next: Next| {
        Box::pin(async move {
            match current_user(req.extensions()) {
                Some(_) => Ok(next.run(req).await),
                None => Err(StatusCode::UNAUTHORIZED),
            }
        })
    }
}
