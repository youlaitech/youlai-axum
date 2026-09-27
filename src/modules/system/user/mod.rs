pub mod dto;
pub mod email;
pub mod service;

use crate::error::{AppError, AppResult};
use crate::modules::auth::extractor;
use crate::modules::auth::extractor::AuthUser;
use crate::modules::auth::permission::require_perm;
use crate::modules::system::user::dto::*;
use crate::modules::system::user::service::UserService;
use crate::response::{PageResult, Result};
use crate::result_code::ResultCode;
use crate::state::AppState;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::Extensions,
    middleware,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use validator::Validate;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/users",
            get(list_users).route_layer(middleware::from_fn(require_perm("sys:user:list"))),
        )
        .route("/api/v1/users/me", get(get_current_user_info))
        .route(
            "/api/v1/users/profile",
            get(get_user_profile).put(update_user_profile),
        )
        .route("/api/v1/users/password", put(change_password))
        .route("/api/v1/users/options", get(list_user_options))
        .route("/api/v1/users/mobile/code", post(send_mobile_code))
        .route(
            "/api/v1/users/mobile",
            put(bind_mobile).delete(unbind_mobile),
        )
        .route("/api/v1/users/email/code", post(send_email_code))
        .route("/api/v1/users/email", put(bind_email).delete(unbind_email))
        .route("/api/v1/users/template", get(download_template))
        .route("/api/v1/users/export", get(export_users))
        .route("/api/v1/users/import", post(import_users))
        .route(
            "/api/v1/users/{id}/form",
            get(get_user_form).route_layer(middleware::from_fn(require_perm("sys:user:update"))),
        )
        .route("/api/v1/users/{id}/status", patch(update_user_status))
        .route(
            "/api/v1/users/{id}/password/reset",
            put(reset_user_password),
        )
        .route(
            "/api/v1/users",
            post(create_user).route_layer(middleware::from_fn(require_perm("sys:user:create"))),
        )
        .route(
            "/api/v1/users/{id}",
            put(update_user).route_layer(middleware::from_fn(require_perm("sys:user:update"))),
        )
        .route(
            "/api/v1/users/{id}",
            delete(delete_users).route_layer(middleware::from_fn(require_perm("sys:user:delete"))),
        )
}

/// 从请求扩展中提取当前登录用户
fn extract_user(
    ext: &Extensions,
) -> AppResult<&crate::modules::auth::principal::Principal> {
    extractor::current_user(ext)
        .ok_or_else(|| AppError::Business(ResultCode::AccessTokenInvalid, None))
}

/// 分页查询用户列表（应用数据权限过滤）
async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<UserQuery>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<PageResult<UserVO>>> {
    let page = UserService::new(&state.db).get_page(&query, &user).await?;
    Ok(Json(PageResult::success(page.list, page.total)))
}

/// 获取用户表单详情
async fn get_user_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Result<UserVO>>> {
    Ok(Json(Result::success(
        UserService::new(&state.db).get_form(id).await?,
    )))
}

/// 创建用户（默认密码 123456）
async fn create_user(
    State(state): State<AppState>,
    Json(form): Json<UserCreate>,
) -> AppResult<Json<Result<i64>>> {
    form.validate()?;
    Ok(Json(Result::success(
        UserService::new(&state.db).create(&form).await?,
    )))
}

/// 更新用户
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(form): Json<UserUpdate>,
) -> AppResult<Json<Result<()>>> {
    form.validate()?;
    let mut f = form;
    f.id = id;
    UserService::new(&state.db).update(&f).await?;
    Ok(Json(Result::success_empty()))
}

/// 删除用户（逻辑删除，支持批量）
async fn delete_users(
    State(state): State<AppState>,
    Path(ids): Path<String>,
) -> AppResult<Json<Result<u64>>> {
    Ok(Json(Result::success(
        UserService::new(&state.db).delete(&ids).await?,
    )))
}

/// 切换用户启用/禁用状态
async fn update_user_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<UserStatusQuery>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .update_status(id, q.status)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 重置用户密码
async fn reset_user_password(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(form): Json<ResetPasswordForm>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .reset_password(id, &form.password)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 获取当前登录用户信息
async fn get_current_user_info(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
) -> AppResult<Json<Result<UserProfileVO>>> {
    let user = extract_user(req.extensions())?;
    Ok(Json(Result::success(
        UserService::new(&state.db)
            .get_current_user(user.user_id)
            .await?,
    )))
}

/// 获取当前用户个人资料
async fn get_user_profile(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
) -> AppResult<Json<Result<UserProfileVO>>> {
    let user = extract_user(req.extensions())?;
    Ok(Json(Result::success(
        UserService::new(&state.db)
            .get_current_user(user.user_id)
            .await?,
    )))
}

/// 更新当前用户个人资料
async fn update_user_profile(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(form): Json<UserProfileUpdate>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .update_profile(user.user_id, &form)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 修改当前用户密码
async fn change_password(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(form): Json<ChangePasswordForm>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .change_password(user.user_id, &form)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 获取用户下拉选项
async fn list_user_options(
    State(state): State<AppState>,
) -> AppResult<Json<Result<Vec<UserOptionVO>>>> {
    Ok(Json(Result::success(
        UserService::new(&state.db).get_options().await?,
    )))
}

/// 发送手机绑定验证码
async fn send_mobile_code(
    State(state): State<AppState>,
    Json(form): Json<MobileCodeForm>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .send_mobile_code(&state.redis, &form.mobile)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 绑定手机号
async fn bind_mobile(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(form): Json<BindMobileForm>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .bind_mobile(&state.redis, user.user_id, &form)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 解绑手机号
async fn unbind_mobile(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
) -> AppResult<Json<Result<()>>> {
    let user = extract_user(req.extensions())?;
    UserService::new(&state.db)
        .unbind_mobile(user.user_id)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 发送邮箱绑定验证码
async fn send_email_code(
    State(state): State<AppState>,
    Json(form): Json<EmailCodeForm>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .send_email_code(&state.redis, &state.settings.mail, &form.email)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 绑定邮箱
async fn bind_email(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(form): Json<BindEmailForm>,
) -> AppResult<Json<Result<()>>> {
    UserService::new(&state.db)
        .bind_email(&state.redis, user.user_id, &form)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 解绑邮箱
async fn unbind_email(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
) -> AppResult<Json<Result<()>>> {
    let user = extract_user(req.extensions())?;
    UserService::new(&state.db)
        .unbind_email(user.user_id)
        .await?;
    Ok(Json(Result::success_empty()))
}

/// 下载用户导入模板（含表头）
async fn download_template() -> AppResult<axum::response::Response> {
    use rust_xlsxwriter::Workbook;
    let mut wb = Workbook::new();
    let sheet = wb.add_worksheet();
    let _ = sheet.write_row(
        0,
        0,
        ["用户名", "昵称", "性别", "手机号码", "邮箱", "角色", "部门"],
    );
    let bytes = wb
        .save_to_buffer()
        .map_err(|e| AppError::Business(ResultCode::UploadFileException, Some(e.to_string())))?;
    Ok(axum::response::IntoResponse::into_response((
        axum::http::StatusCode::OK,
        [
            (
                "Content-Type",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (
                "Content-Disposition",
                "attachment; filename=\"users_template.xlsx\"",
            ),
        ],
        bytes,
    )))
}

/// 导出用户列表
async fn export_users(
    State(state): State<AppState>,
    Query(query): Query<UserQuery>,
    AuthUser(user): AuthUser,
) -> AppResult<axum::response::Response> {
    let page = UserService::new(&state.db).get_page(&query, &user).await?;
    Ok(axum::response::IntoResponse::into_response(Json(
        Result::success(page.list),
    )))
}

/// 导入用户（Excel）：calamine 解析 + 字段校验 + 角色/部门映射
async fn import_users(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<Result<ExcelResultVO>>> {
    let mut data: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Business(ResultCode::UploadFileException, Some(e.to_string())))?
    {
        if field.name().unwrap_or_default() != "file" {
            continue;
        }
        let bytes = field.bytes().await.map_err(|e| {
            AppError::Business(ResultCode::UploadFileException, Some(e.to_string()))
        })?;
        data = Some(bytes.to_vec());
        break;
    }
    let data = data.ok_or_else(|| {
        AppError::Business(
            ResultCode::UploadFileException,
            Some("请求中未包含 file 字段".into()),
        )
    })?;
    Ok(Json(Result::success(
        UserService::new(&state.db).import_users(&data).await?,
    )))
}
