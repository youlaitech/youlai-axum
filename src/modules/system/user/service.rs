//! 用户管理服务

use crate::constants;
use crate::entity;
use crate::error::{AppError, AppResult};
use crate::modules::system::user::dto::*;
use crate::pagination::PageData;
use crate::result_code::ResultCode;
use redis::aio::ConnectionManager;
use sea_orm::sea_query::Expr;
use sea_orm::*;

/// 用户管理服务
pub struct UserService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> UserService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 分页查询用户列表，自动应用数据权限过滤
    pub async fn get_page(
        &self,
        query: &UserQuery,
        user: &crate::modules::auth::principal::Principal,
    ) -> AppResult<PageData<UserVO>> {
        let mut select =
            entity::sys_user::Entity::find().filter(entity::sys_user::Column::IsDeleted.eq(0));

        // 应用数据权限过滤（根据用户角色 data_scope 限制可见范围）
        select = crate::core::data_scope::apply_data_scope(
            self.db,
            select,
            user,
            entity::sys_user::Column::DeptId,
            entity::sys_user::Column::CreateBy,
        )
        .await;

        // 关键词搜索：用户名 / 昵称 / 手机号
        if let Some(ref kw) = query.keywords {
            if !kw.is_empty() {
                select = select.filter(
                    Condition::any()
                        .add(entity::sys_user::Column::Username.like(format!("%{kw}%")))
                        .add(entity::sys_user::Column::Nickname.like(format!("%{kw}%")))
                        .add(entity::sys_user::Column::Mobile.like(format!("%{kw}%"))),
                );
            }
        }
        if let Some(s) = query.status {
            select = select.filter(entity::sys_user::Column::Status.eq(s));
        }
        if let Some(d) = query.dept_id {
            select = select.filter(entity::sys_user::Column::DeptId.eq(d));
        }
        let total = select.clone().count(self.db).await?;
        let users = select
            .order_by_desc(entity::sys_user::Column::CreateTime)
            .paginate(self.db, query.page_size as u64)
            .fetch_page((query.page_num - 1) as u64)
            .await?;
        let vos: Vec<UserVO> = users
            .iter()
            .map(|u| UserVO {
                id: u.id,
                username: u.username.clone(),
                nickname: u.nickname.clone(),
                gender: u.gender.unwrap_or_default(),
                avatar: u.avatar.clone(),
                dept_id: u.dept_id,
                mobile: u.mobile.clone(),
                email: u.email.clone(),
                status: u.status.unwrap_or_default(),
                create_time: u
                    .create_time
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                update_time: u
                    .update_time
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                role_ids: None,
                dept_name: None,
            })
            .collect();
        Ok(PageData::new(vos, total))
    }

    /// 获取用户表单数据（含角色列表）
    pub async fn get_form(&self, id: i64) -> AppResult<UserVO> {
        let u = entity::sys_user::Entity::find_by_id(id)
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(ResultCode::AccountNotFound, Some("用户不存在".into()))
            })?;
        let role_ids: Vec<i64> = entity::sys_user_role::Entity::find()
            .filter(entity::sys_user_role::Column::UserId.eq(id))
            .all(self.db)
            .await?
            .iter()
            .map(|r| r.role_id)
            .collect();
        Ok(UserVO {
            id: u.id,
            username: u.username.clone(),
            nickname: u.nickname.clone(),
            gender: u.gender.unwrap_or_default(),
            avatar: u.avatar.clone(),
            dept_id: u.dept_id,
            mobile: u.mobile.clone(),
            email: u.email.clone(),
            status: u.status.unwrap_or_default(),
            create_time: u
                .create_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            update_time: u
                .update_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            role_ids: Some(role_ids),
            dept_name: None,
        })
    }

    /// 创建用户，默认密码 123456
    pub async fn create(&self, form: &UserCreate) -> AppResult<i64> {
        let exists = entity::sys_user::Entity::find()
            .filter(entity::sys_user::Column::Username.eq(&form.username))
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .is_some();
        if exists {
            return Err(AppError::Business(
                ResultCode::IntegrityConstraintViolation,
                Some("用户名已存在".into()),
            ));
        }

        // bcrypt 加密默认密码
        let pwd = bcrypt::hash(constants::DEFAULT_PASSWORD, bcrypt::DEFAULT_COST).unwrap();
        let active = entity::sys_user::ActiveModel {
            username: Set(Some(form.username.clone())),
            nickname: Set(Some(form.nickname.clone())),
            gender: Set(Some(form.gender)),
            password: Set(Some(pwd)),
            dept_id: Set(form.dept_id),
            mobile: Set(form.mobile.clone()),
            email: Set(form.email.clone()),
            status: Set(Some(form.status)),
            ..Default::default()
        };
        let user = active.insert(self.db).await?;
        if let Some(ref rids) = form.role_ids {
            for &rid in rids {
                entity::sys_user_role::ActiveModel {
                    user_id: Set(user.id),
                    role_id: Set(rid),
                }
                .insert(self.db)
                .await?;
            }
        }
        Ok(user.id)
    }

    /// 更新用户信息及角色
    pub async fn update(&self, form: &UserUpdate) -> AppResult<()> {
        let mut active: entity::sys_user::ActiveModel =
            entity::sys_user::Entity::find_by_id(form.id)
                .filter(entity::sys_user::Column::IsDeleted.eq(0))
                .one(self.db)
                .await?
                .ok_or_else(|| {
                    AppError::Business(ResultCode::AccountNotFound, Some("用户不存在".into()))
                })?
                .into();
        active.nickname = Set(Some(form.nickname.clone()));
        active.gender = Set(Some(form.gender));
        active.dept_id = Set(form.dept_id);
        active.mobile = Set(form.mobile.clone());
        active.email = Set(form.email.clone());
        active.status = Set(Some(form.status));
        active.update(self.db).await?;

        // 先删后插：更新角色关联
        entity::sys_user_role::Entity::delete_many()
            .filter(entity::sys_user_role::Column::UserId.eq(form.id))
            .exec(self.db)
            .await?;
        if let Some(ref rids) = form.role_ids {
            for &rid in rids {
                entity::sys_user_role::ActiveModel {
                    user_id: Set(form.id),
                    role_id: Set(rid),
                }
                .insert(self.db)
                .await?;
            }
        }
        Ok(())
    }

    /// 批量逻辑删除用户
    pub async fn delete(&self, ids: &str) -> AppResult<u64> {
        let id_list: Vec<i64> = ids
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if id_list.is_empty() {
            return Err(AppError::Business(
                ResultCode::UserRequestParameterError,
                None,
            ));
        }
        // 逻辑删除：仅标记 is_deleted=1
        Ok(entity::sys_user::Entity::update_many()
            .col_expr(entity::sys_user::Column::IsDeleted, Expr::value(1))
            .filter(entity::sys_user::Column::Id.is_in(id_list))
            .exec(self.db)
            .await?
            .rows_affected)
    }

    /// 修改用户状态（启用/禁用）
    pub async fn update_status(&self, id: i64, status: i16) -> AppResult<()> {
        entity::sys_user::Entity::update_many()
            .col_expr(entity::sys_user::Column::Status, Expr::value(status))
            .filter(entity::sys_user::Column::Id.eq(id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 重置用户密码
    pub async fn reset_password(&self, id: i64, password: &str) -> AppResult<()> {
        let pwd = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
        entity::sys_user::Entity::update_many()
            .col_expr(entity::sys_user::Column::Password, Expr::value(pwd))
            .filter(entity::sys_user::Column::Id.eq(id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 获取当前登录用户信息（含角色）
    pub async fn get_current_user(&self, user_id: i64) -> AppResult<UserProfileVO> {
        let u = entity::sys_user::Entity::find_by_id(user_id)
            .one(self.db)
            .await?
            .ok_or_else(|| AppError::Business(ResultCode::AccountNotFound, None))?;
        let role_ids: Vec<i64> = entity::sys_user_role::Entity::find()
            .filter(entity::sys_user_role::Column::UserId.eq(user_id))
            .all(self.db)
            .await?
            .iter()
            .map(|r| r.role_id)
            .collect();
        let roles: Vec<String> = if role_ids.is_empty() {
            vec![]
        } else {
            entity::sys_role::Entity::find()
                .filter(entity::sys_role::Column::Id.is_in(role_ids.clone()))
                .all(self.db)
                .await?
                .iter()
                .map(|r| r.code.clone())
                .collect()
        };
        // 通过 role → role_menu → menu 三级关联，查询当前用户所有权限码
        let perms: Vec<String> = if role_ids.is_empty() {
            vec![]
        } else {
            let menu_ids: Vec<i64> = entity::sys_role_menu::Entity::find()
                .filter(entity::sys_role_menu::Column::RoleId.is_in(role_ids.clone()))
                .all(self.db)
                .await?
                .into_iter()
                .map(|rm| rm.menu_id)
                .collect();
            if menu_ids.is_empty() {
                vec![]
            } else {
                entity::sys_menu::Entity::find()
                    .filter(entity::sys_menu::Column::Id.is_in(menu_ids))
                    .all(self.db)
                    .await?
                    .into_iter()
                    .filter_map(|m| m.perm)
                    .filter(|p| !p.is_empty())
                    .collect()
            }
        };
        Ok(UserProfileVO {
            user_id: u.id,
            username: u.username.clone(),
            nickname: u.nickname.clone(),
            gender: u.gender.unwrap_or_default(),
            avatar: u.avatar.clone(),
            dept_id: u.dept_id,
            dept_name: None,
            mobile: u.mobile,
            email: u.email,
            roles,
            perms,
        })
    }

    /// 更新个人资料
    pub async fn update_profile(&self, user_id: i64, form: &UserProfileUpdate) -> AppResult<()> {
        let mut active: entity::sys_user::ActiveModel =
            entity::sys_user::Entity::find_by_id(user_id)
                .one(self.db)
                .await?
                .ok_or_else(|| AppError::Business(ResultCode::AccountNotFound, None))?
                .into();
        if let Some(ref n) = form.nickname {
            active.nickname = Set(Some(n.clone()));
        }
        if let Some(g) = form.gender {
            active.gender = Set(Some(g));
        }
        if let Some(ref m) = form.mobile {
            active.mobile = Set(Some(m.clone()));
        }
        if let Some(ref e) = form.email {
            active.email = Set(Some(e.clone()));
        }
        active.update(self.db).await?;
        Ok(())
    }

    /// 修改当前用户密码
    pub async fn change_password(&self, user_id: i64, form: &ChangePasswordForm) -> AppResult<()> {
        let u = entity::sys_user::Entity::find_by_id(user_id)
            .one(self.db)
            .await?
            .unwrap();
        // 校验原密码
        if !bcrypt::verify(&form.old_password, u.password.as_deref().unwrap_or("")).unwrap_or(false)
        {
            return Err(AppError::Business(
                ResultCode::UserPasswordError,
                Some("原密码错误".into()),
            ));
        }
        // bcrypt 加密新密码
        let new = bcrypt::hash(&form.new_password, bcrypt::DEFAULT_COST).unwrap();
        entity::sys_user::Entity::update_many()
            .col_expr(entity::sys_user::Column::Password, Expr::value(new))
            .filter(entity::sys_user::Column::Id.eq(user_id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 发送手机验证码（随机 6 位，Redis 存 5 分钟，校验未被其他账号绑定）
    pub async fn send_mobile_code(&self, redis: &ConnectionManager, mobile: &str) -> AppResult<()> {
        self.check_contact_unique(entity::sys_user::Column::Mobile, mobile, None)
            .await?;
        let code = format!("{:06}", rand::random::<u32>() % 1000000);
        let key = format!("{}:{}", constants::MOBILE_CODE, mobile);
        let mut r = redis.clone();
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(constants::CODE_TTL)
            .arg(&code)
            .query_async(&mut r)
            .await?;
        tracing::info!("短信验证码已生成，手机号 {}***", &mobile[..3.min(mobile.len())]);
        Ok(())
    }

    /// 绑定手机号：校验验证码 + （可选）登录密码，并确认未被其他账号占用
    pub async fn bind_mobile(
        &self,
        redis: &ConnectionManager,
        user_id: i64,
        form: &BindMobileForm,
    ) -> AppResult<()> {
        self.verify_code(redis, constants::MOBILE_CODE, &form.mobile, &form.code)
            .await?;
        self.verify_password_if_present(user_id, form.password.as_deref())
            .await?;
        self.check_contact_unique(
            entity::sys_user::Column::Mobile,
            &form.mobile,
            Some(user_id),
        )
        .await?;
        let mut r = redis.clone();
        let key = format!("{}:{}", constants::MOBILE_CODE, &form.mobile);
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut r)
            .await
            .unwrap_or(());
        entity::sys_user::Entity::update_many()
            .col_expr(
                entity::sys_user::Column::Mobile,
                Expr::value(form.mobile.clone()),
            )
            .filter(entity::sys_user::Column::Id.eq(user_id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 解绑手机号，校验当前登录密码
    pub async fn unbind_mobile(&self, user_id: i64, password: &str) -> AppResult<()> {
        self.verify_password_if_present(user_id, Some(password))
            .await?;
        entity::sys_user::Entity::update_many()
            .col_expr(entity::sys_user::Column::Mobile, Expr::value(""))
            .filter(entity::sys_user::Column::Id.eq(user_id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 发送邮箱验证码（随机 6 位，Redis 存 5 分钟，校验未被其他账号绑定）
    pub async fn send_email_code(
        &self,
        redis: &ConnectionManager,
        mail_cfg: &crate::config::MailConfig,
        email: &str,
    ) -> AppResult<()> {
        self.check_contact_unique(entity::sys_user::Column::Email, email, None)
            .await?;
        let code = format!("{:06}", rand::random::<u32>() % 1000000);
        let key = format!("{}:{}", constants::EMAIL_CODE, email);
        let mut r = redis.clone();
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(constants::CODE_TTL)
            .arg(&code)
            .query_async(&mut r)
            .await?;
        crate::modules::system::user::email::send_code(email, &code, mail_cfg).await;
        Ok(())
    }

    /// 绑定邮箱：校验验证码 + （可选）登录密码，并确认未被其他账号占用
    pub async fn bind_email(
        &self,
        redis: &ConnectionManager,
        user_id: i64,
        form: &BindEmailForm,
    ) -> AppResult<()> {
        self.verify_code(redis, constants::EMAIL_CODE, &form.email, &form.code)
            .await?;
        self.verify_password_if_present(user_id, form.password.as_deref())
            .await?;
        self.check_contact_unique(entity::sys_user::Column::Email, &form.email, Some(user_id))
            .await?;
        let mut r = redis.clone();
        let key = format!("{}:{}", constants::EMAIL_CODE, &form.email);
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut r)
            .await
            .unwrap_or(());
        entity::sys_user::Entity::update_many()
            .col_expr(
                entity::sys_user::Column::Email,
                Expr::value(form.email.clone()),
            )
            .filter(entity::sys_user::Column::Id.eq(user_id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 解绑邮箱，校验当前登录密码
    pub async fn unbind_email(&self, user_id: i64, password: &str) -> AppResult<()> {
        self.verify_password_if_present(user_id, Some(password))
            .await?;
        entity::sys_user::Entity::update_many()
            .col_expr(entity::sys_user::Column::Email, Expr::value(""))
            .filter(entity::sys_user::Column::Id.eq(user_id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 校验验证码：Redis 中无记录视为过期，不匹配视为错误
    async fn verify_code(
        &self,
        redis: &ConnectionManager,
        prefix: &str,
        contact: &str,
        code: &str,
    ) -> AppResult<()> {
        let key = format!("{}:{}", prefix, contact);
        let mut r = redis.clone();
        let stored: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut r)
            .await
            .ok()
            .flatten();
        match stored {
            Some(s) if s == code => Ok(()),
            Some(_) => Err(AppError::Business(
                ResultCode::UserVerificationCodeError,
                None,
            )),
            None => Err(AppError::Business(
                ResultCode::UserVerificationCodeExpired,
                None,
            )),
        }
    }

    /// 若传入登录密码则校验其与当前账户密码一致
    async fn verify_password_if_present(
        &self,
        user_id: i64,
        password: Option<&str>,
    ) -> AppResult<()> {
        let Some(pwd) = password else {
            return Ok(());
        };
        let u = entity::sys_user::Entity::find_by_id(user_id)
            .one(self.db)
            .await?
            .ok_or_else(|| AppError::Business(ResultCode::AccountNotFound, None))?;
        let ok = bcrypt::verify(pwd, u.password.as_deref().unwrap_or("")).unwrap_or(false);
        if !ok {
            return Err(AppError::Business(ResultCode::UserPasswordError, None));
        }
        Ok(())
    }

    /// 校验联系方式（手机/邮箱）未被其他未删除账户占用
    async fn check_contact_unique(
        &self,
        column: entity::sys_user::Column,
        value: &str,
        exclude_user_id: Option<i64>,
    ) -> AppResult<()> {
        let mut select = entity::sys_user::Entity::find()
            .filter(column.eq(value))
            .filter(entity::sys_user::Column::IsDeleted.eq(0));
        if let Some(id) = exclude_user_id {
            select = select.filter(entity::sys_user::Column::Id.ne(id));
        }
        let exists = select.one(self.db).await?.is_some();
        if exists {
            return Err(AppError::Business(
                ResultCode::IntegrityConstraintViolation,
                Some("该联系方式已被其他账号绑定".into()),
            ));
        }
        Ok(())
    }

    /// 导入用户（Excel）：calamine 解析 + 字段/字典校验 + 角色/部门映射 + 默认密码
    ///
    /// 表头支持中文（用户名/昵称/性别/手机号码/邮箱/角色/部门）或英文（username/nickname/...）。
    /// 校验规则：用户名非空且唯一、昵称非空、手机号非空且符合中国大陆格式；校验通过后以默认密码
    /// 落库，并将角色的「编码或名称」映射为 role_id、部门的「编码或名称」映射为 dept_id。
    pub async fn import_users(&self, data: &[u8]) -> AppResult<ExcelResultVO> {
        use calamine::{open_workbook_from_rs, Reader, Xlsx};
        use std::collections::HashSet;
        use std::io::Cursor;

        let cursor = Cursor::new(data.to_vec());
        let mut wb: Xlsx<Cursor<Vec<u8>>> = open_workbook_from_rs(cursor).map_err(|e| {
            AppError::Business(
                ResultCode::UploadFileException,
                Some(format!("Excel 解析失败: {e}")),
            )
        })?;
        let range = match wb.worksheet_range_at(0) {
            Some(Ok(r)) => r,
            Some(Err(e)) => {
                return Err(AppError::Business(
                    ResultCode::UploadFileException,
                    Some(format!("Excel 读取失败: {e}")),
                ))
            }
            None => {
                return Err(AppError::Business(
                    ResultCode::UploadFileException,
                    Some("Excel 无工作表".into()),
                ))
            }
        };

        // 表头映射：中文/英文 → 列索引
        let headers: Vec<String> = match range.rows().next() {
            Some(r) => r.iter().map(cell_to_string).collect(),
            None => {
                return Ok(ExcelResultVO {
                    valid_count: 0,
                    invalid_count: 0,
                    message_list: vec![],
                })
            }
        };
        let col = |name: &str| -> Option<usize> { headers.iter().position(|h| h.trim() == name) };
        let c_username = col("用户名").or_else(|| col("username"));
        let c_nickname = col("昵称").or_else(|| col("nickname"));
        let c_gender = col("性别").or_else(|| col("genderLabel"));
        let c_mobile = col("手机号码").or_else(|| col("mobile"));
        let c_email = col("邮箱").or_else(|| col("email"));
        let c_role = col("角色").or_else(|| col("roleCodes"));
        let c_dept = col("部门").or_else(|| col("deptCode"));

        // 预加载角色/部门，供编码或名称 → ID 映射
        let roles = entity::sys_role::Entity::find().all(self.db).await?;
        let depts = entity::sys_dept::Entity::find().all(self.db).await?;

        let mut valid_count: u32 = 0;
        let mut invalid_count: u32 = 0;
        let mut message_list: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for (i, row) in range.rows().skip(1).enumerate() {
            let row_no = i + 2; // 第 1 行为表头
            let get = |idx: Option<usize>| -> String {
                idx.and_then(|i| row.get(i))
                    .map(cell_to_string)
                    .unwrap_or_default()
            };
            let username = get(c_username).trim().to_string();
            let nickname = get(c_nickname).trim().to_string();
            let gender_label = get(c_gender).trim().to_string();
            let mobile = get(c_mobile).trim().to_string();
            let email = get(c_email).trim().to_string();
            let role_codes = get(c_role).trim().to_string();
            let dept_code = get(c_dept).trim().to_string();

            // 跳过全空行
            if username.is_empty() && nickname.is_empty() && mobile.is_empty() {
                continue;
            }

            let mut errs: Vec<String> = Vec::new();
            if username.is_empty() {
                errs.push("用户名为空".into());
            } else if seen.contains(&username) {
                errs.push("用户名在本批次中重复".into());
            } else {
                let exists = entity::sys_user::Entity::find()
                    .filter(entity::sys_user::Column::Username.eq(&username))
                    .filter(entity::sys_user::Column::IsDeleted.eq(0))
                    .one(self.db)
                    .await?
                    .is_some();
                if exists {
                    errs.push("用户名已存在".into());
                }
            }
            if nickname.is_empty() {
                errs.push("用户昵称为空".into());
            }
            if mobile.is_empty() {
                errs.push("手机号码为空".into());
            } else if !is_valid_mobile(&mobile) {
                errs.push("手机号码格式不正确".into());
            }

            if !errs.is_empty() {
                invalid_count += 1;
                message_list.push(format!("第{row_no}行数据校验失败：{}", errs.join("；")));
                continue;
            }

            // 映射性别 / 角色 / 部门
            let gender = map_gender(&gender_label);
            let mut role_ids: Vec<i64> = Vec::new();
            if !role_codes.is_empty() {
                for code in role_codes.split(',') {
                    let t = code.trim();
                    if let Some(r) = roles.iter().find(|r| r.code == t || r.name == t) {
                        role_ids.push(r.id);
                    }
                }
                role_ids.sort_unstable();
                role_ids.dedup();
            }
            let dept_id = if dept_code.is_empty() {
                None
            } else {
                depts
                    .iter()
                    .find(|d| d.code == dept_code || d.name == dept_code)
                    .map(|d| d.id)
            };

            let pwd = bcrypt::hash(constants::DEFAULT_PASSWORD, bcrypt::DEFAULT_COST).unwrap();
            let active = entity::sys_user::ActiveModel {
                username: Set(Some(username.clone())),
                nickname: Set(Some(nickname.clone())),
                gender: Set(Some(gender)),
                password: Set(Some(pwd)),
                dept_id: Set(dept_id),
                mobile: Set(if mobile.is_empty() {
                    None
                } else {
                    Some(mobile.clone())
                }),
                email: Set(if email.is_empty() {
                    None
                } else {
                    Some(email.clone())
                }),
                status: Set(Some(1)),
                ..Default::default()
            };
            match active.insert(self.db).await {
                Ok(user) => {
                    for &rid in &role_ids {
                        if let Err(e) = (entity::sys_user_role::ActiveModel {
                            user_id: Set(user.id),
                            role_id: Set(rid),
                        })
                        .insert(self.db)
                        .await
                        {
                            tracing::warn!("用户 {} 角色 {} 关联失败: {}", user.id, rid, e);
                        }
                    }
                    seen.insert(username);
                    valid_count += 1;
                }
                Err(e) => {
                    invalid_count += 1;
                    message_list.push(format!("第{row_no}行数据保存失败：{e}"));
                }
            }
        }

        Ok(ExcelResultVO {
            valid_count,
            invalid_count,
            message_list,
        })
    }

    /// 用户下拉选项
    pub async fn get_options(&self) -> AppResult<Vec<UserOptionVO>> {
        let users = entity::sys_user::Entity::find()
            .filter(entity::sys_user::Column::IsDeleted.eq(0))
            .filter(entity::sys_user::Column::Status.eq(1))
            .all(self.db)
            .await?;
        Ok(users
            .iter()
            .map(|u| UserOptionVO {
                value: u.id,
                label: u.nickname.clone().unwrap_or_default(),
            })
            .collect())
    }
}

/// 将 calamine 单元格转为字符串（去首尾空白）
fn cell_to_string(d: &calamine::Data) -> String {
    match d {
        calamine::Data::String(s) => s.trim().to_string(),
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Float(f) => f.to_string(),
        calamine::Data::Bool(b) => b.to_string(),
        calamine::Data::DateTime(dt) => dt.to_string(),
        calamine::Data::Empty => String::new(),
        _ => String::new(),
    }
}

/// 校验中国大陆手机号：1 开头，第二位 3-9，共 11 位
fn is_valid_mobile(m: &str) -> bool {
    let b = m.as_bytes();
    if b.len() != 11 {
        return false;
    }
    if b[0] != b'1' || !(b'3'..=b'9').contains(&b[1]) {
        return false;
    }
    b.iter().all(u8::is_ascii_digit)
}

/// 性别标签 → 性别值（男=1，女=2，未知=0，默认 1）
fn map_gender(label: &str) -> i16 {
    match label {
        "男" => 1,
        "女" => 2,
        "未知" => 0,
        _ => 1,
    }
}
