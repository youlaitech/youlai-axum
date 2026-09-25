//! 动态表单：定义 CRUD、发布停用、渲染、访问菜单生成与表单数据

pub mod dto;
pub mod service;

use crate::error::AppResult;
use crate::modules::auth::extractor::AuthUser;
use crate::modules::auth::permission::require_perm;
use crate::modules::form::dto::*;
use crate::modules::form::service::FormService;
use crate::response::{PageResult, Result};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde_json::Value;
use validator::Validate;

/// 表单模块共用的 HTTP 客户端（AI 调用）
static HTTP: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

fn service(state: &AppState) -> FormService<'_> {
    FormService::new(&state.db, &state.redis, &HTTP, &state.settings.ai)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // 表单定义
        .route("/api/v1/forms", get(get_form_page))
        .route(
            "/api/v1/forms",
            post(create_form)
                .route_layer(middleware::from_fn(require_perm("form:definition:create"))),
        )
        .route("/api/v1/forms/options", get(list_form_options))
        .route("/api/v1/forms/ai-generate", post(ai_generate_rule))
        .route("/api/v1/forms/{id}/form", get(get_form))
        .route(
            "/api/v1/forms/{id}",
            put(update_form).route_layer(middleware::from_fn(require_perm("form:definition:update"))),
        )
        .route(
            "/api/v1/forms/{id}",
            delete(delete_form)
                .route_layer(middleware::from_fn(require_perm("form:definition:delete"))),
        )
        .route(
            "/api/v1/forms/{id}/publish",
            put(publish_form)
                .route_layer(middleware::from_fn(require_perm("form:definition:update"))),
        )
        .route(
            "/api/v1/forms/{id}/disable",
            put(disable_form)
                .route_layer(middleware::from_fn(require_perm("form:definition:update"))),
        )
        .route("/api/v1/forms/{form_key}/render", get(render))
        // 表单菜单（发布向导）
        .route("/api/v1/forms/{id}/menu", get(get_menu))
        .route(
            "/api/v1/forms/{id}/menu",
            post(save_menu).route_layer(middleware::from_fn(require_perm("form:definition:update"))),
        )
        // 公开表单（匿名，security.ignore_urls 放行）
        .route("/api/v1/forms/public/{form_key}/render", get(public_render))
        .route("/api/v1/forms/public/{form_key}/data", post(save_public_data))
        // 表单数据
        .route(
            "/api/v1/forms/{form_key}/data",
            get(data_page).route_layer(middleware::from_fn(require_perm("form:data:list"))),
        )
        .route("/api/v1/forms/{form_key}/data", post(save_data))
        .route(
            "/api/v1/forms/{form_key}/data/{id}",
            get(data_detail).route_layer(middleware::from_fn(require_perm("form:data:list"))),
        )
        .route(
            "/api/v1/forms/{form_key}/data/{ids}",
            delete(delete_data)
                .route_layer(middleware::from_fn(require_perm("form:data:delete"))),
        )
}

/// 表单定义分页列表
async fn get_form_page(
    State(state): State<AppState>,
    Query(q): Query<FormDefinitionQuery>,
) -> AppResult<Json<PageResult<FormDefinitionVO>>> {
    let p = service(&state).get_page(&q).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 表单定义表单数据
async fn get_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<FormDefinitionForm>>> {
    Ok(Json(Result::success(service(&state).get_form(id).await?)))
}

/// 新增表单定义
async fn create_form(
    State(state): State<AppState>,
    Json(f): Json<FormDefinitionForm>,
) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    Ok(Json(Result::success(service(&state).create(&f).await?)))
}

/// 修改表单定义
async fn update_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut f): Json<FormDefinitionForm>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    f.id = Some(id);
    service(&state).update(&f).await?;
    Ok(Json(Result::success_empty()))
}

/// 删除表单定义
async fn delete_form(
    State(state): State<AppState>,
    Path(ids): Path<String>,
) -> AppResult<Json<Result<()>>> {
    service(&state).delete(&ids).await?;
    Ok(Json(Result::success_empty()))
}

/// 发布表单
async fn publish_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<()>>> {
    service(&state).publish(id).await?;
    Ok(Json(Result::success_empty()))
}

/// 停用表单
async fn disable_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<()>>> {
    service(&state).disable(id).await?;
    Ok(Json(Result::success_empty()))
}

/// 获取表单渲染规则
async fn render(
    State(state): State<AppState>,
    Path(form_key): Path<String>,
) -> AppResult<Json<Result<FormRenderVO>>> {
    Ok(Json(Result::success(service(&state).render(&form_key).await?)))
}

/// 获取公开表单渲染规则（匿名）
async fn public_render(
    State(state): State<AppState>,
    Path(form_key): Path<String>,
) -> AppResult<Json<Result<FormRenderVO>>> {
    Ok(Json(Result::success(
        service(&state).public_render(&form_key).await?,
    )))
}

/// 获取表单菜单配置（向导回显）
async fn get_menu(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<Option<FormMenuVO>>>> {
    Ok(Json(Result::success(service(&state).get_menu(id).await?)))
}

/// 生成表单访问菜单
async fn save_menu(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(f): Json<FormMenuForm>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    service(&state).save_menu(id, &f).await?;
    Ok(Json(Result::success_empty()))
}

/// 审批表单下拉选项
async fn list_form_options(State(state): State<AppState>) -> AppResult<Json<Result<Vec<Value>>>> {
    Ok(Json(Result::success(
        service(&state).workflow_options().await?,
    )))
}

/// AI 生成表单规则
async fn ai_generate_rule(
    State(state): State<AppState>,
    Json(f): Json<FormAiGenerateForm>,
) -> AppResult<Json<Result<String>>> {
    f.validate()?;
    Ok(Json(Result::success(
        service(&state).ai_generate_rule(&f.description).await?,
    )))
}

/// 提交表单数据
async fn save_data(
    State(state): State<AppState>,
    Path(form_key): Path<String>,
    AuthUser(user): AuthUser,
    Json(data): Json<Value>,
) -> AppResult<Json<Result<()>>> {
    service(&state)
        .save_data(&form_key, &data, Some(user.user_id))
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 匿名提交公开表单数据
async fn save_public_data(
    State(state): State<AppState>,
    Path(form_key): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Value>,
) -> AppResult<Json<Result<()>>> {
    let ip = client_ip(&headers);
    service(&state).save_public_data(&form_key, &data, &ip).await?;
    Ok(Json(Result::success_empty()))
}

/// 表单数据分页列表
async fn data_page(
    State(state): State<AppState>,
    Path(form_key): Path<String>,
    Query(q): Query<FormDataQuery>,
) -> AppResult<Json<PageResult<FormDataVO>>> {
    let p = service(&state).data_page(&form_key, &q).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 获取表单数据详情
async fn data_detail(
    State(state): State<AppState>,
    Path((_form_key, id)): Path<(String, i64)>,
) -> AppResult<Json<Result<FormDataVO>>> {
    Ok(Json(Result::success(service(&state).data_detail(id).await?)))
}

/// 删除表单数据
async fn delete_data(
    State(state): State<AppState>,
    Path((_form_key, ids)): Path<(String, String)>,
) -> AppResult<Json<Result<()>>> {
    service(&state).delete_data(&ids).await?;
    Ok(Json(Result::success_empty()))
}

/// 取客户端真实 IP：优先 X-Forwarded-For 首段，否则取 X-Real-IP
fn client_ip(headers: &HeaderMap) -> String {
    for key in ["x-forwarded-for", "x-real-ip"] {
        if let Some(value) = headers.get(key).and_then(|v| v.to_str().ok()) {
            let first = value.split(',').next().unwrap_or("").trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    "unknown".to_string()
}
