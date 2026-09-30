use validator::Validate;
pub mod dto;
pub mod service;

use crate::error::AppResult;
use crate::modules::auth::permission::require_perm;
use crate::modules::system::role::dto::*;
use crate::modules::system::role::service::RoleService;
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
            "/api/v1/roles",
            get(list_roles).route_layer(middleware::from_fn(require_perm("sys:role:list"))),
        )
        .route("/api/v1/roles/options", get(list_role_options))
        .route("/api/v1/roles/code-options", get(list_role_code_options))
        .route(
            "/api/v1/roles",
            post(create_role).route_layer(middleware::from_fn(require_perm("sys:role:create"))),
        )
        .route(
            "/api/v1/roles/{id}/form",
            get(get_role_form).route_layer(middleware::from_fn(require_perm("sys:role:update"))),
        )
        .route(
            "/api/v1/roles/{id}/status",
            put(update_role_status)
                .route_layer(middleware::from_fn(require_perm("sys:role:update"))),
        )
        .route("/api/v1/roles/{id}/menu-ids", get(get_role_menu_ids))
        .route(
            "/api/v1/roles/{id}/menus",
            put(assign_role_menus)
                .route_layer(middleware::from_fn(require_perm("sys:role:assign"))),
        )
        .route(
            "/api/v1/roles/{id}/dept-ids",
            get(get_role_dept_ids)
                .route_layer(middleware::from_fn(require_perm("sys:role:update"))),
        )
        .route(
            "/api/v1/roles/{id}",
            put(update_role).route_layer(middleware::from_fn(require_perm("sys:role:update"))),
        )
        .route(
            "/api/v1/roles/{id}",
            delete(delete_roles).route_layer(middleware::from_fn(require_perm("sys:role:delete"))),
        )
}

/// 分页查询角色列表
async fn list_roles(
    State(state): State<AppState>,
    Query(query): Query<RoleQuery>,
) -> AppResult<Json<PageResult<RoleVO>>> {
    let p = RoleService::new(&state.db).get_page(&query).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 获取角色表单详情
async fn get_role_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<RoleVO>>> {
    Ok(Json(Result::success(
        RoleService::new(&state.db).get_form(id).await?,
    )))
}

/// 创建角色
async fn create_role(
    State(state): State<AppState>,
    Json(f): Json<RoleCreate>,
) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    Ok(Json(Result::success(
        RoleService::new(&state.db).create(&f).await?,
    )))
}

/// 更新角色
async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(f): Json<RoleUpdate>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    let mut f = f;
    f.id = id;
    RoleService::new(&state.db).update(&f).await?;
    Ok(Json(Result::success_empty()))
}

/// 删除角色（支持批量）
async fn delete_roles(
    State(state): State<AppState>,
    Path(ids): Path<String>,
) -> AppResult<Json<Result<u64>>> {
    Ok(Json(Result::success(
        RoleService::new(&state.db).delete(&ids).await?,
    )))
}

/// 切换角色启用/禁用状态
async fn update_role_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<RoleStatusQuery>,
) -> AppResult<Json<Result<()>>> {
    RoleService::new(&state.db)
        .update_status(id, q.status)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 分配角色菜单权限
async fn assign_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(menu_ids): Json<Vec<i64>>,
) -> AppResult<Json<Result<()>>> {
    RoleService::new(&state.db)
        .assign_menus(id, &menu_ids)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 获取角色已分配菜单 ID
async fn get_role_menu_ids(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<Vec<i64>>>> {
    Ok(Json(Result::success(
        RoleService::new(&state.db).get_menu_ids(id).await?,
    )))
}

/// 获取角色数据权限部门 ID
async fn get_role_dept_ids(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<Vec<i64>>>> {
    Ok(Json(Result::success(
        RoleService::new(&state.db).get_dept_ids(id).await?,
    )))
}

/// 获取角色下拉选项
async fn list_role_options(
    State(state): State<AppState>,
) -> AppResult<Json<Result<Vec<RoleOptionVO>>>> {
    Ok(Json(Result::success(
        RoleService::new(&state.db).get_options().await?,
    )))
}

/// 获取角色编码下拉选项
async fn list_role_code_options(
    State(state): State<AppState>,
) -> AppResult<Json<Result<Vec<RoleCodeOptionVO>>>> {
    Ok(Json(Result::success(
        RoleService::new(&state.db).get_code_options().await?,
    )))
}
