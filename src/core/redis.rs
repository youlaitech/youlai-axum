use redis::aio::ConnectionManager;
use crate::core::config::Settings;

/// 初始化异步 Redis 连接
pub async fn init_redis(settings: &Settings) -> anyhow::Result<ConnectionManager> {
    let client = redis::Client::open(settings.redis.url.as_str())?;
    let manager = ConnectionManager::new(client).await?;
    tracing::info!("redis connected: {}", settings.redis.url);
    Ok(manager)
}
