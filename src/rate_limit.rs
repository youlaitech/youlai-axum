use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use redis::aio::ConnectionManager;

use crate::config::RateLimitConfig;
use crate::error::{AppError, AppResult};
use crate::response::Result;
use crate::result_code::ResultCode;
use crate::state::AppState;

/// Redis 滑动窗口限流 Lua 脚本（ZSet ZREMRANGEBYSCORE→ZADD→PEXPIRE→ZCARD，一次往返原子完成）
const SLIDING_WINDOW_LUA: &str = r#"
local key = KEYS[1]
local now = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local member = ARGV[3]
redis.call('ZREMRANGEBYSCORE', key, 0, now - window)
redis.call('ZADD', key, now, member)
redis.call('PEXPIRE', key, window + 1000)
return redis.call('ZCARD', key)
"#;

/// IP 全局限流中间件：按 IP + 登录用户 + 请求路径 维度做滑动窗口计数，超限返回 429。
pub async fn rate_limit_mw(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = &state.settings.rate_limit;
    if !cfg.enabled {
        return next.run(req).await;
    }

    let path = req.uri().path().to_string();
    let ip = extract_ip(&req, addr);
    let scope = extract_scope(&req);
    let (limit, window) = choose_limit(cfg, &path);
    let window_ms = window * 1000;
    let now_ms = Utc::now().timestamp_millis() as u64;
    let key = format!("rate_limit:ip:{ip}:scope:{scope}:{path}");
    let member = uuid::Uuid::new_v4().to_string();

    let mut r = state.redis.clone();
    let eval: redis::RedisResult<i64> = redis::cmd("EVAL")
        .arg(SLIDING_WINDOW_LUA)
        .arg(1)
        .arg(&key)
        .arg(now_ms)
        .arg(window_ms)
        .arg(&member)
        .query_async(&mut r)
        .await;

    match eval {
        Ok(count) => {
            let remaining = limit.saturating_sub(count as u64);
            let reset = ((now_ms + window_ms) / 1000) as u64;
            if count as u64 > limit {
                build_429(limit, remaining, reset)
            } else {
                let mut resp = next.run(req).await;
                insert_headers(&mut resp, limit, remaining, reset);
                resp
            }
        }
        // Redis 不可用时Fail-Open，避免阻断正常业务
        Err(e) => {
            tracing::warn!("rate limit redis error: {e}");
            next.run(req).await
        }
    }
}

fn insert_headers(resp: &mut Response, limit: u64, remaining: u64, reset: u64) {
    let h = resp.headers_mut();
    h.insert("X-RateLimit-Limit", hv(limit));
    h.insert("X-RateLimit-Remaining", hv(remaining));
    h.insert("X-RateLimit-Reset", hv(reset));
}

fn build_429(limit: u64, remaining: u64, reset: u64) -> Response {
    let mut resp = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(Result::<()>::failed(
            ResultCode::RequestConcurrencyLimitExceeded,
            Some("请求过于频繁，请稍后再试".into()),
        )),
    )
        .into_response();
    insert_headers(&mut resp, limit, remaining, reset);
    resp.headers_mut().insert("Retry-After", hv(reset));
    resp
}

/// 从 Authorization 头解析限流维度：携带 Bearer Token 时按 token 隔离配额，
/// 匿名请求统一归入 `anon`，避免不同登录用户共用同一 IP 桶互相误伤。
fn extract_scope(req: &Request) -> String {
    req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "anon".into())
}

/// 优先取反向代理透传的真实客户端 IP，否则取 socket 对端地址
fn extract_ip(req: &Request, addr: SocketAddr) -> String {
    if let Some(fwd) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = fwd.split(',').next() {
            return first.trim().to_string();
        }
    }
    if let Some(real) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return real.trim().to_string();
    }
    addr.ip().to_string()
}

/// 命中严格路径前缀列表时使用更低的阈值，避免高频接口被刷
fn choose_limit(cfg: &RateLimitConfig, path: &str) -> (u64, u64) {
    // 短信验证码接口：1 次/60 秒（接口级限流，防刷短信）
    if path.starts_with("/api/v1/auth/sms/code") {
        return (1, cfg.window_seconds);
    }
    let strict: Vec<&str> = cfg
        .strict_paths
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if strict.iter().any(|p| path.starts_with(*p)) {
        (cfg.strict_limit, cfg.window_seconds)
    } else {
        (cfg.limit, cfg.window_seconds)
    }
}

fn hv(n: u64) -> HeaderValue {
    HeaderValue::from_str(&n.to_string()).unwrap()
}

/// 登录接口按账号限流（5 次/60 秒），防爆破。
///
/// Redis ZSET 滑动窗口（与全局中间件共用同一 Lua 脚本），按 username 维度隔离，
/// 避免同一 IP 下多账号共享计数误伤。
/// 返回 true 表示放行，false 表示已超限（调用方应返回 A0502）。
pub async fn login_rate_allowed(redis: &ConnectionManager, username: &str) -> AppResult<bool> {
    let key = format!("rate_limit:login:{}", username);
    let window_ms: u64 = 60_000;
    let limit: u64 = 5;
    let now_ms = Utc::now().timestamp_millis() as u64;
    let member = uuid::Uuid::new_v4().to_string();
    let mut r = redis.clone();
    let count: i64 = redis::cmd("EVAL")
        .arg(SLIDING_WINDOW_LUA)
        .arg(1)
        .arg(&key)
        .arg(now_ms)
        .arg(window_ms)
        .arg(&member)
        .query_async(&mut r)
        .await?;
    Ok((count as u64) <= limit)
}

/// 登录超限时返回 A0502
pub fn login_rate_limited() -> AppError {
    AppError::Business(
        ResultCode::RequestConcurrencyLimitExceeded,
        Some("请求过于频繁，请稍后再试".into()),
    )
}
