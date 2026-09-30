use crate::response::Result;
use crate::result_code::ResultCode;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};

/// Token 认证中间件：从 Authorization 头提取 Bearer token，解析后注入请求 extensions
pub async fn auth_layer(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    if state
        .settings
        .security
        .ignore_urls_list()
        .iter()
        .any(|u| is_match(u, &path))
    {
        return next.run(req).await;
    }

    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");

    if token.is_empty() {
        tracing::warn!("missing access token for {path}");
        return error_response(
            StatusCode::UNAUTHORIZED,
            ResultCode::AccessTokenInvalid,
            None,
        );
    }

    match state.token_manager.parse_token(token).await {
        Some(user) => {
            req.extensions_mut().insert(user);
            next.run(req).await
        }
        None => {
            tracing::warn!("invalid access token for {path}");
            error_response(
                StatusCode::UNAUTHORIZED,
                ResultCode::AccessTokenInvalid,
                None,
            )
        }
    }
}

fn error_response(status: StatusCode, code: ResultCode, msg: Option<String>) -> Response {
    let body: Result<()> = Result::failed(code, msg);
    (status, Json(body)).into_response()
}

/// 路径匹配：
/// - `**`  后缀：匹配前缀本身及其下任意层级的子路径（如 `/api/v1/auth/sms/**` 匹配 `/api/v1/auth/sms/code` 与 `/api/v1/auth/sms/login`）
/// - `/*`  后缀：匹配前缀下的一级子路径（如 `/swagger-ui/*`）
/// - 其它：精确匹配（忽略结尾斜杠）
fn is_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_end_matches('/');
    if pattern.ends_with("**") {
        let prefix = pattern.trim_end_matches("**").trim_end_matches('/');
        path == prefix || path.starts_with(&format!("{prefix}/"))
    } else if pattern.ends_with("/*") {
        let prefix = pattern.trim_end_matches("/*").trim_end_matches('/');
        path.starts_with(&format!("{prefix}/"))
    } else {
        path == pattern || path == pattern.trim_end_matches('/')
    }
}
