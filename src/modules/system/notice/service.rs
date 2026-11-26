//! 通知公告服务：发布、撤回、已读

use sea_orm::sea_query::Expr;
use sea_orm::*;
use std::collections::HashMap;

use crate::entity;
use crate::error::{AppError, AppResult};
use crate::modules::system::notice::dto::*;
use crate::pagination::PageData;
use crate::result_code::ResultCode;

/// 通知公告服务
pub struct NoticeService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> NoticeService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 通知公告分页列表
    pub async fn get_page(&self, q: &NoticeQuery) -> AppResult<PageData<NoticeVO>> {
        let mut s =
            entity::sys_notice::Entity::find().filter(entity::sys_notice::Column::IsDeleted.eq(0));
        if let Some(ref t) = q.title {
            if !t.is_empty() {
                s = s.filter(entity::sys_notice::Column::Title.like(format!("%{t}%")));
            }
        }
        if let Some(ps) = q.publish_status {
            s = s.filter(entity::sys_notice::Column::PublishStatus.eq(ps));
        }
        let total = s.clone().count(self.db).await?;
        let rows = s
            .order_by_desc(entity::sys_notice::Column::CreateTime)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;
        Ok(PageData::new(
            rows.iter().map(|r| self.to_vo(r)).collect(),
            total,
        ))
    }

    /// 管理端预览通知，不修改阅读状态
    pub async fn get_preview(&self, id: i64) -> AppResult<NoticeVO> {
        let notice = entity::sys_notice::Entity::find_by_id(id)
            .filter(entity::sys_notice::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| AppError::Business(ResultCode::DataNotFound, Some("通知不存在".into())))?;
        let mut vo = self.to_vo(&notice);
        if let Some(publisher_id) = notice.publisher_id {
            vo.publisher_name = entity::sys_user::Entity::find_by_id(publisher_id)
                .select_only()
                .column(entity::sys_user::Column::Nickname)
                .into_tuple()
                .one(self.db)
                .await?
                .and_then(|(nickname,)| nickname);
        }
        Ok(vo)
    }

    /// 获取通知表单
    pub async fn get_form(&self, id: i64) -> AppResult<NoticeVO> {
        let r = entity::sys_notice::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .unwrap();
        Ok(self.to_vo(&r))
    }

    /// sys_notice::Model -> NoticeVO
    fn to_vo(&self, r: &entity::sys_notice::Model) -> NoticeVO {
        NoticeVO {
            id: r.id,
            title: r.title.clone(),
            content: r.content.clone(),
            r#type: r.r#type,
            level: r.level.clone(),
            target_type: r.target_type,
            target_user_ids: r.target_user_ids.clone(),
            publish_status: r.publish_status,
            publisher_id: r.publisher_id,
            publisher_name: None,
            publish_time: r
                .publish_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            revoke_time: r
                .revoke_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            create_time: r.create_time.format("%Y-%m-%d %H:%M:%S").to_string().into(),
            is_read: None,
        }
    }

    /// sys_notice::Model + sys_user_notice::Model -> NoticeVO（带已读状态）
    fn to_vo_with_read(
        &self,
        r: &entity::sys_notice::Model,
        un: &entity::sys_user_notice::Model,
    ) -> NoticeVO {
        let mut vo = self.to_vo(r);
        vo.is_read = Some(un.is_read);
        vo
    }

    /// 将用户 ID 数组转换为逗号分隔字符串存储
    fn join_ids(ids: &Option<Vec<i64>>) -> Option<String> {
        ids.as_ref().map(|v| {
            v.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
    }

    /// 新增通知公告
    pub async fn create(&self, f: &NoticeCreate, user_id: i64) -> AppResult<i64> {
        Ok(entity::sys_notice::ActiveModel {
            title: Set(Some(f.title.clone())),
            content: Set(f.content.clone()),
            r#type: Set(f.r#type),
            level: Set(f.level.clone()),
            target_type: Set(f.target_type),
            target_user_ids: Set(Self::join_ids(&f.target_user_ids)),
            create_by: Set(user_id),
            create_time: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(self.db)
        .await?
        .id)
    }

    /// 更新通知公告
    pub async fn update(&self, f: &NoticeUpdate) -> AppResult<()> {
        let mut a: entity::sys_notice::ActiveModel = entity::sys_notice::Entity::find_by_id(f.id)
            .one(self.db)
            .await?
            .unwrap()
            .into();
        a.title = Set(Some(f.title.clone()));
        a.content = Set(f.content.clone());
        a.r#type = Set(f.r#type);
        a.level = Set(f.level.clone());
        a.target_type = Set(f.target_type);
        a.target_user_ids = Set(Self::join_ids(&f.target_user_ids));
        a.update(self.db).await?;
        Ok(())
    }

    /// 批量删除通知公告（逻辑删除，同时删除 user_notice 关联记录）
    pub async fn delete(&self, ids: &str) -> AppResult<u64> {
        let ids: Vec<i64> = ids
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        // 同时删除 user_notice 关联记录
        entity::sys_user_notice::Entity::delete_many()
            .filter(entity::sys_user_notice::Column::NoticeId.is_in(ids.clone()))
            .exec(self.db)
            .await?;

        Ok(sea_orm::Update::many(entity::sys_notice::Entity)
            .col_expr(entity::sys_notice::Column::IsDeleted, Expr::value(1))
            .filter(entity::sys_notice::Column::Id.is_in(ids))
            .exec(self.db)
            .await?
            .rows_affected)
    }

    /// 发布通知：写未读记录，返回通知体与目标用户 ID 列表
    pub async fn publish(&self, id: i64, publisher_id: i64) -> AppResult<(NoticeVO, Vec<i64>)> {
        let notice = entity::sys_notice::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .ok_or_else(|| AppError::Business(ResultCode::DataNotFound, None))?;
        if notice.publish_status == 1 {
            return Err(AppError::Business(
                ResultCode::SystemError,
                Some("通知已发布".into()),
            ));
        }

        // 删除旧未读记录（支持重新发布）
        let old = entity::sys_user_notice::Entity::find()
            .filter(entity::sys_user_notice::Column::NoticeId.eq(id))
            .all(self.db)
            .await?;
        for r in old {
            entity::sys_user_notice::Entity::delete_by_id(r.id)
                .exec(self.db)
                .await?;
        }

        // 按目标类型筛选用户
        let users: Vec<entity::sys_user::Model> = if notice.target_type == 2 {
            let ids: Vec<i64> = notice
                .target_user_ids
                .as_deref()
                .unwrap_or("")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            entity::sys_user::Entity::find()
                .filter(entity::sys_user::Column::Id.is_in(ids))
                .filter(entity::sys_user::Column::IsDeleted.eq(0))
                .all(self.db)
                .await?
        } else {
            entity::sys_user::Entity::find()
                .filter(entity::sys_user::Column::Status.eq(1))
                .filter(entity::sys_user::Column::IsDeleted.eq(0))
                .all(self.db)
                .await?
        };
        // 目标用户 ID 列表（与实时会话 key 同源，保证定向推送命中）
        let targets: Vec<i64> = users.iter().map(|u| u.id).collect();

        // 批量写入未读记录
        if !users.is_empty() {
            let now = chrono::Utc::now().naive_utc();
            let models: Vec<_> = users
                .iter()
                .map(|u| entity::sys_user_notice::ActiveModel {
                    notice_id: Set(id),
                    user_id: Set(u.id),
                    is_read: Set(0),
                    create_time: Set(now),
                    ..Default::default()
                })
                .collect();
            entity::sys_user_notice::Entity::insert_many(models)
                .exec(self.db)
                .await?;
        }

        sea_orm::Update::many(entity::sys_notice::Entity)
            .col_expr(
                entity::sys_notice::Column::PublishStatus,
                Expr::value(1_i16),
            )
            .col_expr(
                entity::sys_notice::Column::PublishTime,
                Expr::value(chrono::Utc::now().naive_utc()),
            )
            .col_expr(
                entity::sys_notice::Column::PublisherId,
                Expr::value(publisher_id),
            )
            .filter(entity::sys_notice::Column::Id.eq(id))
            .exec(self.db)
            .await?;

        let vo = self.to_vo(&notice);
        Ok((vo, targets))
    }

    /// 撤回通知：置状态、删未读记录
    pub async fn revoke(&self, id: i64) -> AppResult<()> {
        let notice = entity::sys_notice::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .ok_or_else(|| AppError::Business(ResultCode::DataNotFound, None))?;
        if notice.publish_status != 1 {
            return Err(AppError::Business(
                ResultCode::SystemError,
                Some("通知未发布或已撤回".into()),
            ));
        }

        sea_orm::Update::many(entity::sys_notice::Entity)
            .col_expr(
                entity::sys_notice::Column::PublishStatus,
                Expr::value(-1_i16),
            )
            .col_expr(
                entity::sys_notice::Column::RevokeTime,
                Expr::value(chrono::Utc::now().naive_utc()),
            )
            .filter(entity::sys_notice::Column::Id.eq(id))
            .exec(self.db)
            .await?;

        let old = entity::sys_user_notice::Entity::find()
            .filter(entity::sys_user_notice::Column::NoticeId.eq(id))
            .all(self.db)
            .await?;
        for r in old {
            entity::sys_user_notice::Entity::delete_by_id(r.id)
                .exec(self.db)
                .await?;
        }
        Ok(())
    }

    /// 我的通知列表：以 sys_user_notice 为基准，只返回推送给当前用户的通知
    pub async fn get_my_notices(
        &self,
        user_id: i64,
        q: &NoticeQuery,
    ) -> AppResult<PageData<NoticeVO>> {
        let un = entity::sys_user_notice::Entity::find()
            .filter(entity::sys_user_notice::Column::UserId.eq(user_id))
            .filter(entity::sys_user_notice::Column::IsDeleted.eq(0));
        let total = un.clone().count(self.db).await?;
        let rows = un
            .order_by_desc(entity::sys_user_notice::Column::CreateTime)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;

        let list = if rows.is_empty() {
            Vec::new()
        } else {
            let notice_ids: Vec<i64> = rows.iter().map(|r| r.notice_id).collect();
            let notices = entity::sys_notice::Entity::find()
                .filter(entity::sys_notice::Column::Id.is_in(notice_ids))
                .all(self.db)
                .await?;
            let notice_map: HashMap<i64, entity::sys_notice::Model> =
                notices.into_iter().map(|n| (n.id, n)).collect();
            rows.iter()
                .filter_map(|un_row| {
                    notice_map
                        .get(&un_row.notice_id)
                        .map(|n| self.to_vo_with_read(n, un_row))
                })
                .collect()
        };
        Ok(PageData::new(list, total))
    }

    /// 全部标记已读
    pub async fn read_all(&self, user_id: i64) -> AppResult<()> {
        let rows = entity::sys_user_notice::Entity::find()
            .filter(entity::sys_user_notice::Column::UserId.eq(user_id))
            .filter(entity::sys_user_notice::Column::IsRead.eq(0))
            .all(self.db)
            .await?;
        let now = chrono::Utc::now().naive_utc();
        for un in rows {
            let mut am: entity::sys_user_notice::ActiveModel = un.into();
            am.is_read = Set(1);
            am.read_time = Set(Some(now));
            am.update(self.db).await?;
        }
        Ok(())
    }

}
