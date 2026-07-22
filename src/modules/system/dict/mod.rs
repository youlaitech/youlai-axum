use validator::Validate;
pub mod dto;
pub mod service;

use crate::error::AppResult;
use crate::modules::auth::permission::require_perm;
use crate::modules::message::topics::DICT;
use crate::modules::system::dict::dto::*;
use crate::modules::system::dict::service::DictService;
use crate::response::{PageResult, Result};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};

/// 当前毫秒时间戳（SSE 事件用）
fn now_ts() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/dicts",
            get(list_dicts).route_layer(middleware::from_fn(require_perm("sys:dict:list"))),
        )
        .route("/api/v1/dicts/options", get(list_dict_options))
        .route(
            "/api/v1/dicts",
            post(create_dict).route_layer(middleware::from_fn(require_perm("sys:dict:create"))),
        )
        .route("/api/v1/dicts/{id}/form", get(get_dict_form))
        .route(
            "/api/v1/dicts/{id}",
            put(update_dict).route_layer(middleware::from_fn(require_perm("sys:dict:update"))),
        )
        .route(
            "/api/v1/dicts/{id}",
            delete(delete_dicts).route_layer(middleware::from_fn(require_perm("sys:dict:delete"))),
        )
        .route(
            "/api/v1/dicts/{dict_code}/items",
            get(list_dict_items)
                .route_layer(middleware::from_fn(require_perm("sys:dict-item:list"))),
        )
        .route(
            "/api/v1/dicts/{dict_code}/items/options",
            get(list_dict_item_options),
        )
        .route(
            "/api/v1/dicts/{dict_code}/items/{item_id}/form",
            get(get_dict_item_form),
        )
        .route(
            "/api/v1/dicts/{dict_code}/items/{item_id}",
            put(update_dict_item)
                .route_layer(middleware::from_fn(require_perm("sys:dict-item:update"))),
        )
        .route(
            "/api/v1/dicts/{dict_code}/items",
            post(create_dict_item)
                .route_layer(middleware::from_fn(require_perm("sys:dict-item:create"))),
        )
        .route(
            "/api/v1/dicts/{dict_code}/items/{item_id}",
            delete(delete_dict_items)
                .route_layer(middleware::from_fn(require_perm("sys:dict-item:delete"))),
        )
}

async fn list_dicts(
    State(state): State<AppState>,
    Query(q): Query<DictQuery>,
) -> AppResult<Json<PageResult<DictVO>>> {
    let p = DictService::new(&state.db).get_page(&q).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 获取字典下拉选项
async fn list_dict_options(State(state): State<AppState>) -> AppResult<Json<Result<Vec<DictVO>>>> {
    Ok(Json(Result::success(
        DictService::new(&state.db).get_options().await?,
    )))
}

async fn get_dict_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<DictVO>>> {
    Ok(Json(Result::success(
        DictService::new(&state.db).get_form(id).await?,
    )))
}

async fn create_dict(
    State(state): State<AppState>,
    Json(f): Json<DictCreate>,
) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    let id = DictService::new(&state.db).create(&f).await?;
    state
        .message_hub
        .broadcast(
            DICT,
            serde_json::json!({ "dictCode": f.dict_code, "timestamp": now_ts() }),
        )
        .await;
    Ok(Json(Result::success(id)))
}

/// 更新字典
async fn update_dict(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(f): Json<DictUpdate>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    let mut f = f;
    f.id = id;
    DictService::new(&state.db).update(&f).await?;
    state
        .message_hub
        .broadcast(
            DICT,
            serde_json::json!({ "dictCode": f.dict_code, "timestamp": now_ts() }),
        )
        .await;
    Ok(Json(Result::success_empty()))
}

async fn delete_dicts(
    State(state): State<AppState>,
    Path(ids): Path<String>,
) -> AppResult<Json<Result<u64>>> {
    let codes = DictService::new(&state.db).get_codes_by_ids(&ids).await?;
    let n = DictService::new(&state.db).delete(&ids).await?;
    let ts = now_ts();
    for code in codes {
        state
            .message_hub
            .broadcast(
                DICT,
                serde_json::json!({ "dictCode": code, "timestamp": ts }),
            )
            .await;
    }
    Ok(Json(Result::success(n)))
}

async fn list_dict_items(
    State(state): State<AppState>,
    Path(dict_code): Path<String>,
    Query(q): Query<DictItemQuery>,
) -> AppResult<Json<PageResult<DictItemVO>>> {
    let p = DictService::new(&state.db)
        .get_items(&dict_code, &q)
        .await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

async fn list_dict_item_options(
    State(state): State<AppState>,
    Path(dict_code): Path<String>,
) -> AppResult<Json<Result<Vec<DictItemVO>>>> {
    Ok(Json(Result::success(
        DictService::new(&state.db)
            .get_item_options(&dict_code)
            .await?,
    )))
}

async fn get_dict_item_form(
    State(state): State<AppState>,
    Path((_dict_code, item_id)): Path<(String, i64)>,
) -> AppResult<Json<Result<DictItemVO>>> {
    Ok(Json(Result::success(
        DictService::new(&state.db).get_item_form(item_id).await?,
    )))
}

async fn create_dict_item(
    State(state): State<AppState>,
    Path(dict_code): Path<String>,
    Json(f): Json<DictItemCreate>,
) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    let id = DictService::new(&state.db)
        .create_item(&dict_code, &f)
        .await?;
    state
        .message_hub
        .broadcast(
            DICT,
            serde_json::json!({ "dictCode": dict_code, "timestamp": now_ts() }),
        )
        .await;
    Ok(Json(Result::success(id)))
}

async fn update_dict_item(
    State(state): State<AppState>,
    Path((_dict_code, item_id)): Path<(String, i64)>,
    Json(f): Json<DictItemUpdate>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    let mut f = f;
    f.id = item_id;
    DictService::new(&state.db).update_item(&f).await?;
    state
        .message_hub
        .broadcast(
            DICT,
            serde_json::json!({ "dictCode": _dict_code, "timestamp": now_ts() }),
        )
        .await;
    Ok(Json(Result::success_empty()))
}

/// 删除字典项（支持批量）
async fn delete_dict_items(
    State(state): State<AppState>,
    Path((dict_code, item_ids)): Path<(String, String)>,
) -> AppResult<Json<Result<u64>>> {
    let n = DictService::new(&state.db).delete_items(&item_ids).await?;
    state
        .message_hub
        .broadcast(
            DICT,
            serde_json::json!({ "dictCode": dict_code, "timestamp": now_ts() }),
        )
        .await;
    Ok(Json(Result::success(n)))
}
