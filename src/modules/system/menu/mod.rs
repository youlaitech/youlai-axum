use validator::Validate;
pub mod dto;
pub mod service;

use crate::error::AppResult;
use crate::modules::auth::extractor::AuthUser;
use crate::modules::auth::permission::require_perm;
use crate::modules::system::menu::dto::*;
use crate::modules::system::menu::service::MenuService;
use crate::response::Result;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, RawQuery, State},
    middleware,
    routing::{get, patch, post, put},
    Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/menus",
            get(list_menus).route_layer(middleware::from_fn(require_perm("sys:menu:list"))),
        )
        .route("/api/v1/menus/options", get(list_menu_options))
        .route("/api/v1/menus/routes", get(get_routes))
        .route(
            "/api/v1/menus",
            post(create_menu).route_layer(middleware::from_fn(require_perm("sys:menu:create"))),
        )
        .route("/api/v1/menus/{id}/form", get(get_menu_form))
        .route("/api/v1/menus/{id}", put(update_menu).delete(delete_menu))
        .route("/api/v1/menus/{id}", patch(update_menu_visible))
}

async fn list_menus(State(state): State<AppState>) -> AppResult<Json<Result<Vec<MenuVO>>>> {
    Ok(Json(Result::success(
        MenuService::new(&state.db).get_tree().await?,
    )))
}

async fn get_menu_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<MenuVO>>> {
    Ok(Json(Result::success(
        MenuService::new(&state.db).get_form(id).await?,
    )))
}

/// 创建菜单
async fn create_menu(
    State(state): State<AppState>,
    Json(f): Json<MenuCreate>,
) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    Ok(Json(Result::success(
        MenuService::new(&state.db).create(&f).await?,
    )))
}

/// 更新菜单
async fn update_menu(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(f): Json<MenuUpdate>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    let mut f = f;
    f.id = id;
    MenuService::new(&state.db).update(&f).await?;
    Ok(Json(Result::success_empty()))
}

async fn delete_menu(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<()>>> {
    MenuService::new(&state.db).delete(id).await?;
    Ok(Json(Result::success_empty()))
}

/// 切换菜单显示/隐藏
async fn update_menu_visible(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<MenuVisibleQuery>,
) -> AppResult<Json<Result<()>>> {
    MenuService::new(&state.db)
        .update_visible(id, q.visible)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 获取当前用户可访问的路由菜单
async fn get_routes(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<Result<Vec<RouteVO>>>> {
    Ok(Json(Result::success(
        MenuService::new(&state.db).get_routes(&user).await?,
    )))
}

/// 获取菜单下拉选项
async fn list_menu_options(
    State(state): State<AppState>,
    RawQuery(raw): RawQuery,
) -> AppResult<Json<Result<Vec<MenuOptionVO>>>> {
    let types = parse_types_param(raw.as_deref());
    Ok(Json(Result::success(
        MenuService::new(&state.db).get_options(types).await?,
    )))
}

/// 解析 types 查询参数：支持 `types=C,M` 与重复键 `types=C&types=M`
fn parse_types_param(raw: Option<&str>) -> Option<Vec<String>> {
    let mut types: Vec<String> = Vec::new();
    for pair in raw?.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key != "types" {
            continue;
        }
        for item in value.split(',') {
            let item = item.trim();
            if !item.is_empty() && !types.iter().any(|t| t == item) {
                types.push(item.to_string());
            }
        }
    }
    if types.is_empty() {
        None
    } else {
        Some(types)
    }
}
