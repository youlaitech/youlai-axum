//! 应用装配：资源初始化、Router 聚合、全局 layer、启动与关闭。

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use redis::aio::ConnectionManager;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::Settings;
use crate::modules::auth::middleware::auth_layer;
use crate::modules::auth::routes as auth_routes;
use crate::modules::auth::token::create_token_manager;
use crate::modules::codegen::routes as codegen_routes;
use crate::modules::file::routes as file_routes;
use crate::modules::sse::hub::MessageHub;
use crate::modules::sse::routes as sse_routes;
use crate::modules::system::routes as system_routes;
use crate::openapi::swagger_ui;
use crate::rate_limit::rate_limit_mw;
use crate::state::AppState;

/// 初始化数据库连接池（PostgreSQL）
async fn init_db(settings: &Settings) -> anyhow::Result<DatabaseConnection> {
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

/// 初始化异步 Redis 连接
async fn init_redis(settings: &Settings) -> anyhow::Result<ConnectionManager> {
    let client = redis::Client::open(settings.redis.url.as_str())?;
    let manager = ConnectionManager::new(client).await?;
    tracing::info!("redis connected: {}", settings.redis.url);
    Ok(manager)
}

/// 聚合业务 Router、OpenAPI 与全局 layer，返回未绑定监听端口的 Router
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(auth_routes())
        .merge(system_routes())
        .merge(sse_routes())
        .merge(file_routes())
        .merge(codegen_routes())
        .merge(swagger_ui())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit_mw,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_layer,
        ))
        .with_state(state)
}

/// 加载配置、初始化资源并启动 HTTP 服务
pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let settings = Arc::new(Settings::load()?);
    tracing::info!(
        "{} v{} starting",
        settings.app.name,
        env!("CARGO_PKG_VERSION")
    );

    let db = init_db(&settings).await?;
    let redis = init_redis(&settings).await?;
    let token_manager = Arc::from(create_token_manager(&settings, redis.clone()));
    let message_hub = Arc::new(MessageHub::new());

    let state = AppState {
        db,
        redis,
        token_manager,
        message_hub,
        settings: settings.clone(),
    };

    let app = build_router(state);

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{}", addr);
    tracing::info!("swagger: http://{}/swagger-ui", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.unwrap() };
    tokio::select! { _ = ctrl_c => {} }
    tracing::info!("shutting down");
}
