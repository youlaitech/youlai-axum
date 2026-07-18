//! 认证服务：登录、令牌刷新、权限组装

use crate::constants;
use crate::entity;
use crate::error::{AppError, AppResult};
use crate::modules::auth::dto::*;
use crate::modules::auth::principal::Principal;
use crate::modules::auth::token::DynTokenManager;
use crate::result_code::ResultCode;
use redis::aio::ConnectionManager;
use sea_orm::*;

/// 认证服务：登录、令牌刷新、权限组装
pub struct AuthService<'a> {
    db: &'a DatabaseConnection,
    redis: &'a ConnectionManager,
    token_manager: &'a DynTokenManager,
}

impl<'a> AuthService<'a> {
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

    /// 账号密码登录，校验验证码
    pub async fn login(&self, req: &LoginRequest) -> AppResult<LoginResp> {
        // 验证码校验（若前端传入）
        if let (Some(cid), Some(code)) = (&req.captcha_id, &req.captcha_code) {
            if !crate::modules::auth::captcha::verify(self.redis, cid, code).await {
                return Err(AppError::Business(
                    ResultCode::UserVerificationCodeError,
                    None,
                ));
            }
        }

        // 查用户（未逻辑删除的）
        let user = entity::sys_user::Entity::find()
            .filter(entity::sys_user::Column::Username.eq(&req.username))
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(ResultCode::AccountNotFound, Some("用户名或密码错误".into()))
            })?;

        // 账户已冻结
        if user.status.unwrap_or_default() != 1 {
            return Err(AppError::Business(ResultCode::AccountFrozen, None));
        }

        // bcrypt 密码校验
        if !bcrypt::verify(&req.password, user.password.as_deref().unwrap_or("")).unwrap_or(false) {
            return Err(AppError::Business(ResultCode::UserPasswordError, None));
        }

        // 组装权限并生成 Token
        let details = self.build_user_details(&user).await?;
        let token = self.token_manager.generate_token(&details).await;
        Ok(LoginResp {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_in: token.expires_in,
            token_type: token.token_type,
        })
    }

    /// 短信验证码登录
    pub async fn login_by_sms(&self, req: &SmsLoginRequest) -> AppResult<LoginResp> {
        // Redis 校验短信验证码
        let key = format!("sms:{}", &req.mobile);
        let mut r = self.redis.clone();
        let stored: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut r)
            .await
            .ok()
            .flatten();
        if stored.as_deref() != Some(&req.code) {
            return Err(AppError::Business(
                ResultCode::UserVerificationCodeError,
                None,
            ));
        }
        // 验证码一次性消费
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut r)
            .await
            .unwrap_or(());

        // 按手机号查用户
        let user = entity::sys_user::Entity::find()
            .filter(entity::sys_user::Column::Mobile.eq(&req.mobile))
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(ResultCode::AccountNotFound, Some("手机号未注册".into()))
            })?;

        let details = self.build_user_details(&user).await?;
        let token = self.token_manager.generate_token(&details).await;
        Ok(LoginResp {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_in: token.expires_in,
            token_type: token.token_type,
        })
    }

    /// 发送短信验证码（模拟实现，存 Redis）
    pub async fn send_sms_code(&self, req: &SendSmsRequest) -> AppResult<()> {
        let code = format!("{:06}", rand::random::<u32>() % 1000000);
        let key = format!("sms:{}", &req.mobile);
        let mut r = self.redis.clone();
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(300)
            .arg(&code)
            .query_async(&mut r)
            .await?;
        tracing::info!("sms code sent to {}: {}", &req.mobile, code);
        Ok(())
    }

    /// 使用 refresh token 换取新的 access token
    pub async fn refresh_token(&self, req: &RefreshTokenRequest) -> AppResult<LoginResp> {
        let token = self
            .token_manager
            .refresh_token(&req.refresh_token)
            .await
            .ok_or_else(|| AppError::Business(ResultCode::RefreshTokenInvalid, None))?;
        Ok(LoginResp {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_in: token.expires_in,
            token_type: token.token_type,
        })
    }

    /// 按手机号查找或创建用户并签发令牌（微信小程序登录复用）
    pub async fn login_by_mobile(&self, mobile: &str) -> AppResult<LoginResp> {
        let user = self.find_or_create_user_by_mobile(mobile).await?;
        self.generate_token_for(&user).await
    }

    /// 按 openid 查找已绑定用户（静默登录复用）
    pub async fn find_user_by_openid(
        &self,
        openid: &str,
    ) -> AppResult<Option<entity::sys_user::Model>> {
        let social = entity::sys_user_social::Entity::find()
            .filter(entity::sys_user_social::Column::Platform.eq("WECHAT_MINI"))
            .filter(entity::sys_user_social::Column::Openid.eq(openid))
            .one(self.db)
            .await?;
        if let Some(s) = social {
            let user = entity::sys_user::Entity::find_by_id(s.user_id)
                .filter(entity::sys_user::Column::IsDeleted.eq(0))
                .one(self.db)
                .await?;
            Ok(user)
        } else {
            Ok(None)
        }
    }

    /// 按手机号查找用户，不存在则创建（默认昵称"微信用户"，分配 GUEST 角色）
    pub async fn find_or_create_user_by_mobile(
        &self,
        mobile: &str,
    ) -> AppResult<entity::sys_user::Model> {
        if let Some(u) = entity::sys_user::Entity::find()
            .filter(entity::sys_user::Column::Mobile.eq(mobile))
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
        {
            return Ok(u);
        }
        let username = format!(
            "wx_{}",
            &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
        );
        let now = chrono::Utc::now().naive_utc();
        let password = bcrypt::hash(crate::constants::DEFAULT_PASSWORD, 10).unwrap_or_default();
        let user = entity::sys_user::ActiveModel {
            username: Set(Some(username)),
            nickname: Set(Some("微信用户".into())),
            mobile: Set(Some(mobile.into())),
            password: Set(Some(password)),
            gender: Set(Some(1)),
            status: Set(Some(1)),
            is_deleted: Set(Some(0)),
            create_time: Set(Some(now)),
            update_time: Set(Some(now)),
            ..Default::default()
        }
        .insert(self.db)
        .await?;

        // 分配 GUEST 角色（按 code 查询，查不到则跳过）
        if let Some(role) = entity::sys_role::Entity::find()
            .filter(entity::sys_role::Column::Code.eq("GUEST"))
            .filter(entity::sys_role::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
        {
            let _: Result<_, _> = entity::sys_user_role::ActiveModel {
                user_id: Set(user.id),
                role_id: Set(role.id),
                ..Default::default()
            }
            .insert(self.db)
            .await;
        }
        Ok(user)
    }

    /// 绑定或更新微信小程序社交账号（openid）
    pub async fn bind_wechat_openid(
        &self,
        user_id: i64,
        openid: &str,
        unionid: Option<&str>,
        session_key: Option<&str>,
    ) -> AppResult<()> {
        let existing = entity::sys_user_social::Entity::find()
            .filter(entity::sys_user_social::Column::Platform.eq("WECHAT_MINI"))
            .filter(entity::sys_user_social::Column::Openid.eq(openid))
            .one(self.db)
            .await?;
        let now = chrono::Utc::now().naive_utc();
        if let Some(s) = existing {
            let mut am: entity::sys_user_social::ActiveModel = s.into();
            am.user_id = Set(user_id);
            if session_key.is_some() {
                am.session_key = Set(session_key.map(|x| x.to_string()));
            }
            am.update_time = Set(Some(now));
            am.update(self.db).await.ok();
        } else {
            let _: Result<_, _> = entity::sys_user_social::ActiveModel {
                user_id: Set(user_id),
                platform: Set("WECHAT_MINI".into()),
                openid: Set(openid.into()),
                unionid: Set(unionid.map(|x| x.to_string())),
                session_key: Set(session_key.map(|x| x.to_string())),
                verified: Set(1),
                create_time: Set(Some(now)),
                update_time: Set(Some(now)),
                ..Default::default()
            }
            .insert(self.db)
            .await;
        }
        Ok(())
    }

    /// 组装权限并签发令牌
    async fn generate_token_for(&self, user: &entity::sys_user::Model) -> AppResult<LoginResp> {
        let details = self.build_user_details(user).await?;
        let token = self.token_manager.generate_token(&details).await;
        Ok(LoginResp {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_in: token.expires_in,
            token_type: token.token_type,
        })
    }

    /// 查询用户角色、权限、数据权限，构建 Principal
    async fn build_user_details(
        &self,
        user: &entity::sys_user::Model,
    ) -> AppResult<Principal> {
        // 1. 查询用户关联的角色
        let user_roles: Vec<entity::sys_user_role::Model> = entity::sys_user_role::Entity::find()
            .filter(entity::sys_user_role::Column::UserId.eq(user.id))
            .all(self.db)
            .await?;
        let role_ids: Vec<i64> = user_roles.iter().map(|r| r.role_id).collect();

        // 2. 查询角色详情（过滤已停用和已删除）
        let roles = if role_ids.is_empty() {
            vec![]
        } else {
            entity::sys_role::Entity::find()
                .filter(entity::sys_role::Column::Id.is_in(role_ids))
                .filter(entity::sys_role::Column::Status.eq(1))
                .filter(entity::sys_role::Column::IsDeleted.eq(0))
                .all(self.db)
                .await?
        };

        let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();
        // 判断是否超级管理员：拥有 ROOT 角色，或用户名为 admin/root
        let is_root = role_codes.contains(&constants::ROOT_ROLE_CODE.to_string())
            || user.username.as_deref() == Some("admin")
            || user.username.as_deref() == Some("root");

        // 3. 组装按钮权限标识（超级管理员拥有通配符权限）
        let perms = if is_root {
            vec!["*:*:*".to_string()]
        } else {
            let role_ids: Vec<i64> = roles.iter().map(|r| r.id).collect();
            let role_menus: Vec<entity::sys_role_menu::Model> =
                entity::sys_role_menu::Entity::find()
                    .filter(entity::sys_role_menu::Column::RoleId.is_in(role_ids))
                    .all(self.db)
                    .await?;
            let menu_ids: Vec<i64> = role_menus.iter().map(|rm| rm.menu_id).collect();
            if menu_ids.is_empty() {
                vec![]
            } else {
                // 从菜单表提取非空 perm 字段
                entity::sys_menu::Entity::find()
                    .filter(entity::sys_menu::Column::Id.is_in(menu_ids))
                    .all(self.db)
                    .await?
                    .iter()
                    .filter_map(|m| m.perm.clone())
                    .filter(|p| !p.is_empty())
                    .collect()
            }
        };

        // 4. 组装数据权限（CUSTOM 需要查询 sys_role_dept 表）
        let mut data_scopes: Vec<crate::modules::auth::principal::RoleDataScope> = vec![];
        for r in &roles {
            let scope = r.data_scope.unwrap_or(1) as i32;
            // scope == 5 (CUSTOM)：查询自定义部门 ID 列表
            let dept_ids = if scope == 5 {
                entity::sys_role_dept::Entity::find()
                    .filter(entity::sys_role_dept::Column::RoleId.eq(r.id))
                    .all(self.db)
                    .await
                    .ok()
                    .map(|list| list.iter().map(|d| d.dept_id).collect())
            } else {
                None
            };
            data_scopes.push(crate::modules::auth::principal::RoleDataScope {
                data_scope: scope,
                dept_ids,
            });
        }

        Ok(Principal {
            user_id: user.id,
            username: user.username.clone().unwrap_or_default(),
            password: user.password.clone().unwrap_or_default(),
            enabled: user.status.unwrap_or_default() == 1,
            dept_id: user.dept_id,
            data_scopes,
            roles: role_codes,
            perms,
            is_root,
            avatar: user.avatar.clone(),
        })
    }

    /// 按用户 ID 加载并组装认证详情（扫码登录换发会话令牌复用）
    pub async fn get_principal_by_id(&self, user_id: i64) -> AppResult<Principal> {
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
        self.build_user_details(&user).await
    }
}
