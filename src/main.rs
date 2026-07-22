//! youlai-axum — Axum + SeaORM + PostgreSQL 管理后台

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    youlai_axum::run().await
}
