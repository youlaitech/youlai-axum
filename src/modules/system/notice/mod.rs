use validator::Validate;
pub mod dto;
pub mod service;

use crate::error::{AppError, AppResult};
use crate::modules::auth::extractor;
use crate::modules::auth::extractor::AuthUser;
use crate::modules::auth::permission::require_perm;
use crate::modules::realtime::topics::{EVENT_NOTICE_CHANGED, TOPIC_NOTICE};
use crate::modules::system::notice::dto::*;
use crate::modules::system::notice::service::NoticeService;
use crate::response::{PageResult, Result};
use crate::result_code::ResultCode;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::Extensions,
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/notices",
            get(list_notices).route_layer(middleware::from_fn(require_perm("sys:notice:list"))),
        )
        .route("/api/v1/notices/my", get(list_my_notices))
        .route("/api/v1/notices/read-all", put(read_all))
        .route("/api/v1/notices/{id}/detail", get(get_notice_detail))
        .route(
            "/api/v1/notices/{id}/form",
            get(get_notice_form)
                .route_layer(middleware::from_fn(require_perm("sys:notice:update"))),
        )
        .route(
            "/api/v1/notices/{id}/publish",
            put(publish_notice)
                .route_layer(middleware::from_fn(require_perm("sys:notice:publish"))),
        )
        .route(
            "/api/v1/notices/{id}/revoke",
            put(revoke_notice)
                .route_layer(middleware::from_fn(require_perm("sys:notice:revoke"))),
        )
        .route(
            "/api/v1/notices",
            post(create_notice).route_layer(middleware::from_fn(require_perm("sys:notice:create"))),
        )
        .route(
            "/api/v1/notices/{id}",
            put(update_notice)
                .route_layer(middleware::from_fn(require_perm("sys:notice:update"))),
        )
        .route(
            "/api/v1/notices/{id}",
            delete(delete_notices)
                .route_layer(middleware::from_fn(require_perm("sys:notice:delete"))),
        )
}

/// 从请求扩展中提取当前登录用户
fn extract_user(
    ext: &Extensions,
) -> AppResult<&crate::modules::auth::principal::Principal> {
    extractor::current_user(ext)
        .ok_or_else(|| AppError::Business(ResultCode::AccessTokenInvalid, None))
}

/// 分页查询通知公告列表
async fn list_notices(
    State(state): State<AppState>,
    Query(q): Query<NoticeQuery>,
) -> AppResult<Json<PageResult<NoticeVO>>> {
    let p = NoticeService::new(&state.db).get_page(&q).await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 查询当前用户收到的通知
async fn list_my_notices(
    State(state): State<AppState>,
    Query(q): Query<NoticeQuery>,
    req: axum::http::Request<axum::body::Body>,
) -> AppResult<Json<PageResult<NoticeVO>>> {
    let user = extract_user(req.extensions())?;
    let p = NoticeService::new(&state.db)
        .get_my_notices(user.user_id, &q)
        .await?;
    Ok(Json(PageResult::success(p.list, p.total)))
}

/// 标记当前用户所有通知为已读
async fn read_all(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
) -> AppResult<Json<Result<()>>> {
    let user = extract_user(req.extensions())?;
    NoticeService::new(&state.db).read_all(user.user_id).await?;
    Ok(Json(Result::success_empty()))
}

/// 通知详情（登录用户查看时标记为已读）
async fn get_notice_detail(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<Result<NoticeVO>>> {
    let db = &state.db;
    // 标记为已读
    let rows = crate::entity::sys_user_notice::Entity::find()
        .filter(crate::entity::sys_user_notice::Column::NoticeId.eq(id))
        .filter(crate::entity::sys_user_notice::Column::UserId.eq(user.user_id))
        .filter(crate::entity::sys_user_notice::Column::IsRead.eq(0))
        .all(db)
        .await?;
    let now = chrono::Utc::now().naive_utc();
    for un in rows {
        let mut am: crate::entity::sys_user_notice::ActiveModel = un.into();
        am.is_read = Set(1);
        am.read_time = Set(Some(now));
        am.update(db).await?;
    }
    Ok(Json(Result::success(
        NoticeService::new(db).get_form(id).await?,
    )))
}

async fn get_notice_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<NoticeVO>>> {
    Ok(Json(Result::success(
        NoticeService::new(&state.db).get_form(id).await?,
    )))
}

async fn create_notice(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(f): Json<NoticeCreate>,
) -> AppResult<Json<Result<i64>>> {
    f.validate()?;
    Ok(Json(Result::success(
        NoticeService::new(&state.db).create(&f, user.user_id).await?,
    )))
}

async fn update_notice(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(f): Json<NoticeUpdate>,
) -> AppResult<Json<Result<()>>> {
    f.validate()?;
    let mut f = f;
    f.id = id;
    NoticeService::new(&state.db).update(&f).await?;
    Ok(Json(Result::success_empty()))
}

/// 删除通知公告（支持批量）
async fn delete_notices(
    State(state): State<AppState>,
    Path(ids): Path<String>,
) -> AppResult<Json<Result<u64>>> {
    Ok(Json(Result::success(
        NoticeService::new(&state.db).delete(&ids).await?,
    )))
}

/// 发布通知公告
async fn publish_notice(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<Result<()>>> {
    let (_, target_ids) = NoticeService::new(&state.db)
        .publish(id, user.user_id)
        .await?;
    // 仅推送给在线的目标用户（定向推送按 user_id 命中实时会话）
    let online: std::collections::HashSet<i64> =
        state.message_hub.get_online_users().await.into_iter().collect();
    for uid in &target_ids {
        if online.contains(uid) {
            state
                .message_hub
                .send_to_user(
                    *uid,
                    EVENT_NOTICE_CHANGED,
                    serde_json::json!({ "noticeId": id.to_string(), "action": "published" }),
                )
                .await;
        }
    }
    Ok(Json(Result::success_empty()))
}

/// 撤回通知公告
async fn revoke_notice(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<()>>> {
    NoticeService::new(&state.db).revoke(id).await?;
    state
        .message_hub
        .broadcast(
            TOPIC_NOTICE,
            EVENT_NOTICE_CHANGED,
            serde_json::json!({ "noticeId": id.to_string(), "action": "revoked" }),
        )
        .await;
    Ok(Json(Result::success_empty()))
}
