//! 扫码登录服务：生成票据、状态流转、换发会话令牌

use crate::constants;
use crate::entity;
use crate::error::{AppError, AppResult};
use crate::modules::auth::dto::LoginResp;
use crate::modules::auth::qrcode::model::*;
use crate::modules::auth::qrcode::util::mask_nickname;
use crate::modules::auth::service::AuthService;
use crate::modules::auth::token::DynTokenManager;
use crate::result_code::ResultCode;
use redis::aio::ConnectionManager;
use sea_orm::*;
use uuid::Uuid;

/// 票据默认有效期（秒），二维码在此时间内未走完流程即由 Redis TTL 自动清理
const DEFAULT_EXPIRE_SECONDS: i64 = 300;
/// 状态流转时若 Redis 剩余 TTL 小于该值，补足到此值，避免临界点票据在下一步操作前被清掉
const MIN_REMAIN_SECONDS: i64 = 30;

/// 扫码登录服务：状态流转的合法性由 require_status / require_same_user 在写入前拦截，
/// 非法迁移直接抛业务异常，因此同一 ticket 被多端并发操作时不需要分布式锁。
/// PC 端换取会话时调用 AuthService 签发令牌，与账号密码登录使用相同的令牌格式和 Redis 会话存储。
pub struct QrCodeLoginService<'a> {
    db: &'a DatabaseConnection,
    redis: &'a ConnectionManager,
    token_manager: &'a DynTokenManager,
}

impl<'a> QrCodeLoginService<'a> {
    pub fn new(
        db: &'a DatabaseConnection,
        redis: &'a ConnectionManager,
        token_manager: &'a DynTokenManager,
    ) -> Self {
        Self {
            db,
            redis,
            token_manager,
        }
    }

    /// 生成票据，未登录调用
    pub async fn generate(&self, client_ip: &str) -> AppResult<QrCodeGenerateVO> {
        let ticket = Uuid::new_v4().to_string().replace('-', "");
        let ctx = QrCodeLoginContext {
            ticket: ticket.clone(),
            status: STATUS_WAITING.to_string(),
            user_id: None,
            nickname: None,
            avatar: None,
            created_at: Some(chrono::Utc::now().timestamp_millis()),
            scanned_at: None,
            confirmed_at: None,
            client_ip: Some(client_ip.to_string()),
        };
        self.save(&ctx, DEFAULT_EXPIRE_SECONDS).await?;
        Ok(QrCodeGenerateVO {
            ticket,
            expire_seconds: DEFAULT_EXPIRE_SECONDS,
        })
    }

    /// 查询状态，未登录调用
    pub async fn status(&self, ticket: &str) -> AppResult<QrCodeStatusVO> {
        let ctx = self.load_or_throw(ticket).await?;
        let secs = self.remain_seconds(ticket).await;
        Ok(self.to_vo(&ctx, secs))
    }

    /// APP 标记已扫码：须处于 WAITING
    pub async fn scan(&self, ticket: &str, user_id: i64) -> AppResult<QrCodeStatusVO> {
        let mut ctx = self.load_or_throw(ticket).await?;
        self.require_status(&ctx, STATUS_WAITING)?;
        self.fill_user_info(&mut ctx, user_id).await?;
        ctx.status = STATUS_SCANNED.to_string();
        ctx.scanned_at = Some(chrono::Utc::now().timestamp_millis());
        self.save(&ctx, self.refresh_ttl(ticket).await).await?;
        let secs = self.remain_seconds(ticket).await;
        Ok(self.to_vo(&ctx, secs))
    }

    /// APP 确认登录：须处于 SCANNED，操作者须为扫码本人
    pub async fn confirm(&self, ticket: &str, user_id: i64) -> AppResult<QrCodeStatusVO> {
        let mut ctx = self.load_or_throw(ticket).await?;
        self.require_status(&ctx, STATUS_SCANNED)?;
        self.require_same_user(&ctx, user_id)?;
        ctx.status = STATUS_CONFIRMED.to_string();
        ctx.confirmed_at = Some(chrono::Utc::now().timestamp_millis());
        self.save(&ctx, self.refresh_ttl(ticket).await).await?;
        let secs = self.remain_seconds(ticket).await;
        Ok(self.to_vo(&ctx, secs))
    }

    /// APP 取消登录：WAITING/SCANNED/CONFIRMED 允许，已扫码后仅扫码本人可取消
    pub async fn cancel(&self, ticket: &str, user_id: i64) -> AppResult<QrCodeStatusVO> {
        let mut ctx = self.load_or_throw(ticket).await?;
        if ctx.status != STATUS_WAITING
            && ctx.status != STATUS_SCANNED
            && ctx.status != STATUS_CONFIRMED
        {
            return Err(AppError::Business(ResultCode::QrCodeStatusIllegal, None));
        }
        if ctx.status != STATUS_WAITING && ctx.user_id.is_some() {
            self.require_same_user(&ctx, user_id)?;
        }
        ctx.status = STATUS_CANCELED.to_string();
        self.save(&ctx, self.refresh_ttl(ticket).await).await?;
        let secs = self.remain_seconds(ticket).await;
        Ok(self.to_vo(&ctx, secs))
    }

    /// PC 端用票据换取会话令牌：须处于 CONFIRMED，成功后票据置 LOGGED_IN
    pub async fn login(&self, ticket: &str) -> AppResult<LoginResp> {
        let mut ctx = self.load_or_throw(ticket).await?;
        self.require_status(&ctx, STATUS_CONFIRMED)?;
        let user_id = ctx.user_id.ok_or_else(|| {
            AppError::Business(
                ResultCode::AccountNotFound,
                Some("扫码用户不存在".into()),
            )
        })?;
        let user = entity::sys_user::Entity::find_by_id(user_id)
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(ResultCode::AccountNotFound, Some("用户不存在".into()))
            })?;
        if user.status.unwrap_or_default() != 1 {
            return Err(AppError::Business(ResultCode::AccountFrozen, None));
        }
        // 调用账号登录的权限组装与令牌签发
        let auth = AuthService::new(self.db, self.redis, self.token_manager);
        let details = auth.get_principal_by_id(user_id).await?;
        let token = self.token_manager.generate_token(&details).await;
        ctx.status = STATUS_LOGGED_IN.to_string();
        let remain = self.remain_seconds(ticket).await;
        let ttl = if remain < MIN_REMAIN_SECONDS {
            MIN_REMAIN_SECONDS
        } else {
            remain
        };
        self.save(&ctx, ttl).await?;
        Ok(LoginResp {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_in: token.expires_in,
            token_type: token.token_type,
        })
    }

    // ======================== private ========================

    /// 读取票据上下文，票据为空、不存在或已过期都视为 QR_CODE_NOT_FOUND
    async fn load_or_throw(&self, ticket: &str) -> AppResult<QrCodeLoginContext> {
        if ticket.is_empty() {
            return Err(AppError::Business(ResultCode::QrCodeNotFound, None));
        }
        let mut r = self.redis.clone();
        let key = format!("{}:{}", constants::redis_key::QR_CODE_PREFIX, ticket);
        let val: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut r)
            .await
            .ok()
            .flatten();
        let val = val.ok_or_else(|| AppError::Business(ResultCode::QrCodeNotFound, None))?;
        serde_json::from_str(&val)
            .map_err(|_| AppError::Business(ResultCode::QrCodeNotFound, None))
    }

    /// 当前状态必须等于期望状态，否则说明流程被跳步或重复操作
    fn require_status(&self, ctx: &QrCodeLoginContext, expected: &str) -> AppResult<()> {
        if ctx.status != expected {
            return Err(AppError::Business(ResultCode::QrCodeStatusIllegal, None));
        }
        Ok(())
    }

    /// 操作者必须是当初扫码的那个用户，防止 A 扫码 B 确认
    fn require_same_user(&self, ctx: &QrCodeLoginContext, user_id: i64) -> AppResult<()> {
        if ctx.user_id.is_none() || ctx.user_id != Some(user_id) {
            return Err(AppError::Business(ResultCode::QrCodeUserMismatch, None));
        }
        Ok(())
    }

    /// 扫码时把当前 APP 用户的昵称、头像写进上下文，供 PC 端 status 展示
    async fn fill_user_info(&self, ctx: &mut QrCodeLoginContext, user_id: i64) -> AppResult<()> {
        let user = entity::sys_user::Entity::find_by_id(user_id)
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(ResultCode::AccountNotFound, Some("用户不存在".into()))
            })?;
        ctx.user_id = Some(user_id);
        ctx.nickname = user.nickname.clone();
        ctx.avatar = user.avatar.clone();
        Ok(())
    }

    /// 重写整个上下文并刷新 TTL
    async fn save(&self, ctx: &QrCodeLoginContext, ttl: i64) -> AppResult<()> {
        let mut r = self.redis.clone();
        let key = format!("{}:{}", constants::redis_key::QR_CODE_PREFIX, &ctx.ticket);
        let json = serde_json::to_string(ctx).map_err(|_| {
            AppError::Business(ResultCode::SystemError, Some("序列化扫码上下文失败".into()))
        })?;
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg(&json)
            .query_async(&mut r)
            .await?;
        Ok(())
    }

    /// 票据在 Redis 中的剩余秒数，取不到或已过期返回 0
    async fn remain_seconds(&self, ticket: &str) -> i64 {
        let mut r = self.redis.clone();
        let key = format!("{}:{}", constants::redis_key::QR_CODE_PREFIX, ticket);
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut r)
            .await
            .unwrap_or(-2);
        if ttl < 0 {
            0
        } else {
            ttl
        }
    }

    /// 状态流转后写回的 TTL：维持剩余时间，不足最小补足值则补足
    async fn refresh_ttl(&self, ticket: &str) -> i64 {
        let remain = self.remain_seconds(ticket).await;
        if remain < MIN_REMAIN_SECONDS {
            MIN_REMAIN_SECONDS
        } else {
            remain
        }
    }

    /// 上下文转前端 VO，昵称脱敏、用户信息仅在扫码后暴露
    fn to_vo(&self, ctx: &QrCodeLoginContext, expire_seconds: i64) -> QrCodeStatusVO {
        let mut vo = QrCodeStatusVO {
            ticket: ctx.ticket.clone(),
            status: ctx.status.clone(),
            nickname: None,
            avatar: None,
            expire_seconds,
        };
        // WAITING 阶段谁都能查状态，此时不能泄露用户信息；扫码/确认后才回传脱敏昵称与头像
        if ctx.status == STATUS_SCANNED || ctx.status == STATUS_CONFIRMED {
            vo.nickname = ctx.nickname.as_ref().map(|n| mask_nickname(n));
            vo.avatar = ctx.avatar.clone();
        }
        vo
    }
}
