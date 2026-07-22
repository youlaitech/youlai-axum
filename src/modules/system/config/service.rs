//! 系统配置服务

use crate::entity;
use crate::error::AppResult;
use crate::modules::system::config::dto::*;
use crate::pagination::PageData;
use redis::aio::ConnectionManager;
use sea_orm::sea_query::Expr;
use sea_orm::*;

/// 系统配置服务
pub struct ConfigService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ConfigService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 配置分页列表
    pub async fn get_page(&self, q: &ConfigQuery) -> AppResult<PageData<ConfigVO>> {
        let mut s =
            entity::sys_config::Entity::find().filter(entity::sys_config::Column::IsDeleted.eq(0));
        if let Some(ref kw) = q.keywords {
            if !kw.is_empty() {
                s = s.filter(
                    Condition::any()
                        .add(entity::sys_config::Column::ConfigName.like(format!("%{kw}%")))
                        .add(entity::sys_config::Column::ConfigKey.like(format!("%{kw}%"))),
                );
            }
        }
        let total = s.clone().count(self.db).await?;
        let rows = s
            .order_by_desc(entity::sys_config::Column::CreateTime)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;
        Ok(PageData::new(
            rows.iter()
                .map(|r| ConfigVO {
                    id: r.id,
                    config_name: r.config_name.clone(),
                    config_key: r.config_key.clone(),
                    config_value: r.config_value.clone(),
                    remark: r.remark.clone(),
                    create_time: r
                        .create_time
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                })
                .collect(),
            total,
        ))
    }

    /// 获取配置表单
    pub async fn get_form(&self, id: i64) -> AppResult<ConfigVO> {
        let r = entity::sys_config::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .unwrap();
        Ok(ConfigVO {
            id: r.id,
            config_name: r.config_name.clone(),
            config_key: r.config_key.clone(),
            config_value: r.config_value.clone(),
            remark: r.remark.clone(),
            create_time: r
                .create_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        })
    }

    /// 新增配置
    pub async fn create(&self, f: &ConfigCreate) -> AppResult<i64> {
        Ok(entity::sys_config::ActiveModel {
            config_name: Set(f.config_name.clone()),
            config_key: Set(f.config_key.clone()),
            config_value: Set(f.config_value.clone()),
            remark: Set(f.remark.clone()),
            ..Default::default()
        }
        .insert(self.db)
        .await?
        .id)
    }

    /// 更新配置
    pub async fn update(&self, f: &ConfigUpdate) -> AppResult<()> {
        let mut a: entity::sys_config::ActiveModel = entity::sys_config::Entity::find_by_id(f.id)
            .one(self.db)
            .await?
            .unwrap()
            .into();
        a.config_name = Set(f.config_name.clone());
        a.config_key = Set(f.config_key.clone());
        a.config_value = Set(f.config_value.clone());
        a.remark = Set(f.remark.clone());
        a.update(self.db).await?;
        Ok(())
    }

    /// 删除配置（逻辑删除）
    pub async fn delete(&self, id: i64) -> AppResult<()> {
        entity::sys_config::Entity::update_many()
            .col_expr(entity::sys_config::Column::IsDeleted, Expr::value(1))
            .filter(entity::sys_config::Column::Id.eq(id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 刷新配置缓存
    pub async fn refresh_cache(&self, _redis: &ConnectionManager) -> AppResult<()> {
        entity::sys_config::Entity::find()
            .filter(entity::sys_config::Column::IsDeleted.eq(0))
            .all(self.db)
            .await?;
        Ok(())
    }
}
