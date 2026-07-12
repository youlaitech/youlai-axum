//! youlai-rust — Axum + SeaORM + PostgreSQL 管理后台

mod common;
mod core;
mod entity;
mod framework;
mod modules;

use std::net::SocketAddr;
use std::sync::Arc;
use axum::Router;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::core::{config::Settings, database, redis, state::AppState};
use crate::framework::{apidoc, security};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let settings = Arc::new(Settings::load()?);
    tracing::info!("{} v0.1.0 starting", settings.app.name);

    let db = database::init_db(&settings).await?;
    let redis = redis::init_redis(&settings).await?;
    let token_manager = Arc::from(security::token::create_token_manager(&settings, redis.clone()));
    let sse = Arc::new(framework::sse::manager::SseManager::new());

    let state = AppState {
        db,
        redis,
        token_manager,
        sse,
        settings: settings.clone(),
    };

    let app = Router::new()
        .merge(modules::auth::routes())
        .merge(modules::system::routes())
        .merge(modules::message::routes())
        .merge(modules::file::routes())
        .merge(modules::codegen::routes())
        .merge(apidoc::swagger_ui())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::framework::middleware::rate_limit::rate_limit_mw,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security::layer::auth_layer,
        ))
        .with_state(state);

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{}", addr);
    tracing::info!("swagger: http://{}/swagger-ui", addr);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.unwrap() };
    tokio::select! { _ = ctrl_c => {} }
    tracing::info!("shutting down");
}
