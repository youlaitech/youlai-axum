//! 一次性握手票据：Redis 存储，取出即删除

use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 票据有效期（秒）
pub const TICKET_TTL: u64 = 30;

/// Redis 键前缀
const TICKET_KEY_PREFIX: &str = "realtime:ticket:";

/// 票据承载的握手授权内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketGrant {
    /// 本地用户ID
    pub user_id: i64,
    /// 用户名
    pub username: String,
    /// 访问令牌，握手时复核
    pub access_token: String,
    /// 订阅主题
    pub topics: Vec<String>,
    /// 过期时间戳（秒）
    pub expires_at: i64,
}

/// 签发票据，键只保存票据本身，随机性足够
pub async fn issue_ticket(
    redis: &ConnectionManager,
    user_id: i64,
    username: &str,
    access_token: &str,
    topics: &[String],
) -> Result<String, redis::RedisError> {
    let ticket = format!(
        "{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let grant = TicketGrant {
        user_id,
        username: username.to_string(),
        access_token: access_token.to_string(),
        topics: topics.to_vec(),
        expires_at: chrono::Utc::now().timestamp() + TICKET_TTL as i64,
    };
    let payload = serde_json::to_string(&grant).unwrap_or_default();
    let mut conn = redis.clone();
    let _: () = redis::cmd("SETEX")
        .arg(ticket_key(&ticket))
        .arg(TICKET_TTL)
        .arg(payload)
        .query_async(&mut conn)
        .await?;
    Ok(ticket)
}

/// 取出并删除票据，无效、已使用或已过期返回 None
pub async fn consume_ticket(
    redis: &ConnectionManager,
    ticket: &str,
) -> Option<TicketGrant> {
    if ticket.len() < 43 || ticket.len() > 256 {
        return None;
    }
    let mut conn = redis.clone();
    let raw: Option<String> = redis::cmd("GETDEL")
        .arg(ticket_key(ticket))
        .query_async(&mut conn)
        .await
        .unwrap_or(None);
    let Some(raw) = raw else {
        return None;
    };
    let grant: TicketGrant = serde_json::from_str(&raw).ok()?;
    if grant.expires_at <= chrono::Utc::now().timestamp() {
        return None;
    }
    Some(grant)
}

/// 构造 Redis 键
fn ticket_key(ticket: &str) -> String {
    format!("{}{}", TICKET_KEY_PREFIX, ticket)
}
