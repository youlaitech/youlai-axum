use validator::Validate;
pub mod dto;
pub mod service;

use crate::error::AppResult;
use crate::modules::auth::permission::require_perm;
use crate::modules::system::config::dto::*;
use crate::modules::system::config::service::ConfigService;
use crate::response::{PageResult, Result};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/configs",
            get(list_configs).route_layer(middleware::from_fn(require_perm("sys:config:list"))),
        )
        .route(
            "/api/v1/configs/refresh",
            put(refresh_cache).route_layer(middleware::from_fn(require_perm("sys:config:refresh"))),
        )
        .route(
            "/api/v1/configs",
            post(create_config).route_layer(middleware::from_fn(require_perm("sys:config:create"))),
        )
        .route("/api/v1/configs/{id}/form", get(get_config_form))
        .route(
            "/api/v1/configs/{id}",
            put(update_config).route_layer(middleware::from_fn(require_perm("sys:config:update"))),
        )
        .route(
            "/api/v1/configs/{id}",
            delete(delete_config).route_layer(middleware::from_fn(require_perm("sys:config:delete"))),
        )
}

/// 分页查询系统配置列表
async fn list_configs(
    State(state): State<AppState>,
    Query(q): Query<ConfigQuery>,
) -> AppResult<Json<PageResult<ConfigVO>>> {
    let p = ConfigService::new(&state.db).get_page(&q).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 刷新系统配置缓存
async fn refresh_cache(State(state): State<AppState>) -> AppResult<Json<Result<()>>> {
    ConfigService::new(&state.db)
        .refresh_cache(&state.redis)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 获取系统配置表单详情
async fn get_config_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<ConfigVO>>> {
    Ok(Json(Result::success(
        ConfigService::new(&state.db).get_form(id).await?,
    )))
}

/// 创建系统配置
async fn create_config(
    State(state): State<AppState>,
    Json(f): Json<ConfigCreate>,
) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    Ok(Json(Result::success(
        ConfigService::new(&state.db).create(&f).await?,
    )))
}

async fn update_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(f): Json<ConfigUpdate>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    let mut f = f;
    f.id = id;
    ConfigService::new(&state.db).update(&f).await?;
    Ok(Json(Result::success_empty()))
}

/// 删除系统配置
async fn delete_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<()>>> {
    ConfigService::new(&state.db).delete(id).await?;
    Ok(Json(Result::success_empty()))
}
