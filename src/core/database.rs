use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use crate::core::config::Settings;

/// 初始化数据库连接池（PostgreSQL）
pub async fn init_db(settings: &Settings) -> anyhow::Result<DatabaseConnection> {
    let mut opt = ConnectOptions::new(&settings.database.url);
    opt.max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .connect_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(600))
        .sqlx_logging(true)
        .sqlx_logging_level(tracing::log::LevelFilter::Info);

    let db = Database::connect(opt).await?;
    tracing::info!(
        "database connected: {}",
        settings.database.url.split('@').last().unwrap_or("?")
    );
    Ok(db)
}
