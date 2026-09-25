//! 字典管理服务：字典类型 + 字典项

use crate::entity;
use crate::error::{AppError, AppResult};
use crate::result_code::ResultCode;
use crate::modules::system::dict::dto::*;
use crate::pagination::PageData;
use sea_orm::sea_query::Expr;
use sea_orm::*;

/// 字典管理服务（含字典项）
pub struct DictService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> DictService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 字典类型分页列表
    pub async fn get_page(&self, q: &DictQuery) -> AppResult<PageData<DictVO>> {
        let mut s =
            entity::sys_dict::Entity::find().filter(entity::sys_dict::Column::IsDeleted.eq(0));
        if let Some(ref kw) = q.keywords {
            if !kw.is_empty() {
                s = s.filter(
                    Condition::any()
                        .add(entity::sys_dict::Column::Name.like(format!("%{kw}%")))
                        .add(entity::sys_dict::Column::DictCode.like(format!("%{kw}%"))),
                );
            }
        }
        let total = s.clone().count(self.db).await?;
        let rows = s
            .order_by_desc(entity::sys_dict::Column::CreateTime)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;
        Ok(PageData::new(
            rows.iter()
                .map(|r| DictVO {
                    id: r.id,
                    dict_code: r.dict_code.clone(),
                    name: r.name.clone(),
                    status: r.status,
                    remark: r.remark.clone(),
                    create_time: r
                        .create_time
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                })
                .collect(),
            total,
        ))
    }

    /// 字典类型下拉选项
    pub async fn get_options(&self) -> AppResult<Vec<DictVO>> {
        let rows = entity::sys_dict::Entity::find()
            .filter(entity::sys_dict::Column::IsDeleted.eq(0))
            .filter(entity::sys_dict::Column::Status.eq(1))
            .all(self.db)
            .await?;
        Ok(rows
            .iter()
            .map(|r| DictVO {
                id: r.id,
                dict_code: r.dict_code.clone(),
                name: r.name.clone(),
                status: r.status,
                remark: r.remark.clone(),
                create_time: None,
            })
            .collect())
    }

    /// 获取字典类型表单
    pub async fn get_form(&self, id: i64) -> AppResult<DictVO> {
        let r = entity::sys_dict::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .unwrap();
        Ok(DictVO {
            id: r.id,
            dict_code: r.dict_code.clone(),
            name: r.name.clone(),
            status: r.status,
            remark: r.remark.clone(),
            create_time: r
                .create_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        })
    }

    /// 新增字典类型
    pub async fn create(&self, f: &DictCreate) -> AppResult<i64> {
        Ok(entity::sys_dict::ActiveModel {
            dict_code: Set(Some(f.dict_code.clone())),
            name: Set(Some(f.name.clone())),
            status: Set(f.status.unwrap_or(1)),
            remark: Set(f.remark.clone()),
            ..Default::default()
        }
        .insert(self.db)
        .await?
        .id)
    }

    /// 更新字典类型
    pub async fn update(&self, f: &DictUpdate) -> AppResult<()> {
        let mut a: entity::sys_dict::ActiveModel = entity::sys_dict::Entity::find_by_id(f.id)
            .one(self.db)
            .await?
            .unwrap()
            .into();
        a.dict_code = Set(Some(f.dict_code.clone()));
        a.name = Set(Some(f.name.clone()));
        a.status = Set(f.status.unwrap_or(1));
        a.remark = Set(f.remark.clone());
        a.update(self.db).await?;
        Ok(())
    }

    /// 批量删除字典类型（逻辑删除，级联逻辑删除其字典项）
    pub async fn delete(&self, ids: &str) -> AppResult<u64> {
        // 先取字典编码：删除后再查会被逻辑删除条件过滤
        let codes = self.get_codes_by_ids(ids).await?;
        let ids: Vec<i64> = ids
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let affected = entity::sys_dict::Entity::update_many()
            .col_expr(entity::sys_dict::Column::IsDeleted, Expr::value(1))
            .filter(entity::sys_dict::Column::Id.is_in(ids))
            .exec(self.db)
            .await?
            .rows_affected;
        if !codes.is_empty() {
            entity::sys_dict_item::Entity::update_many()
                .col_expr(entity::sys_dict_item::Column::IsDeleted, Expr::value(1))
                .filter(entity::sys_dict_item::Column::DictCode.is_in(codes))
                .exec(self.db)
                .await?;
        }
        Ok(affected)
    }

    /// 字典项分页列表
    pub async fn get_items(
        &self,
        dict_code: &str,
        q: &DictItemQuery,
    ) -> AppResult<PageData<DictItemVO>> {
        let s = entity::sys_dict_item::Entity::find()
            .filter(entity::sys_dict_item::Column::DictCode.eq(dict_code))
            .filter(entity::sys_dict_item::Column::IsDeleted.eq(0));
        let total = s.clone().count(self.db).await?;
        let rows = s
            .order_by_asc(entity::sys_dict_item::Column::Sort)
            .order_by_asc(entity::sys_dict_item::Column::Id)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;
        Ok(PageData::new(
            rows.iter()
                .map(|r| DictItemVO {
                    id: r.id,
                    dict_code: r.dict_code.clone(),
                    value: r.value.clone(),
                    label: r.label.clone(),
                    tag_type: r.tag_type.clone(),
                    status: r.status,
                    sort: r.sort,
                })
                .collect(),
            total,
        ))
    }

    /// 字典项下拉选项
    pub async fn get_item_options(&self, dict_code: &str) -> AppResult<Vec<DictItemVO>> {
        let rows = entity::sys_dict_item::Entity::find()
            .filter(entity::sys_dict_item::Column::DictCode.eq(dict_code))
            .filter(entity::sys_dict_item::Column::Status.eq(1))
            .filter(entity::sys_dict_item::Column::IsDeleted.eq(0))
            .order_by_asc(entity::sys_dict_item::Column::Sort)
            .order_by_asc(entity::sys_dict_item::Column::Id)
            .all(self.db)
            .await?;
        Ok(rows
            .iter()
            .map(|r| DictItemVO {
                id: r.id,
                dict_code: r.dict_code.clone(),
                value: r.value.clone(),
                label: r.label.clone(),
                tag_type: r.tag_type.clone(),
                status: r.status,
                sort: r.sort,
            })
            .collect())
    }

    /// 获取字典项表单
    pub async fn get_item_form(&self, id: i64) -> AppResult<DictItemVO> {
        let r = entity::sys_dict_item::Entity::find_by_id(id)
            .filter(entity::sys_dict_item::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(
                    ResultCode::UserRequestParameterError,
                    Some("字典项不存在".into()),
                )
            })?;
        Ok(DictItemVO {
            id: r.id,
            dict_code: r.dict_code.clone(),
            value: r.value.clone(),
            label: r.label.clone(),
            tag_type: r.tag_type.clone(),
            status: r.status,
            sort: r.sort,
        })
    }

    /// 新增字典项
    pub async fn create_item(&self, dict_code: &str, f: &DictItemCreate) -> AppResult<i64> {
        self.check_item_value_unique(dict_code, &f.value, None).await?;
        Ok(entity::sys_dict_item::ActiveModel {
            dict_code: Set(Some(dict_code.to_string())),
            value: Set(Some(f.value.clone())),
            label: Set(Some(f.label.clone())),
            tag_type: Set(f.tag_type.clone()),
            status: Set(f.status.unwrap_or(1)),
            sort: Set(f.sort.unwrap_or(0)),
            ..Default::default()
        }
        .insert(self.db)
        .await?
        .id)
    }

    /// 更新字典项
    pub async fn update_item(&self, f: &DictItemUpdate) -> AppResult<()> {
        let existing = entity::sys_dict_item::Entity::find_by_id(f.id)
            .filter(entity::sys_dict_item::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(
                    ResultCode::UserRequestParameterError,
                    Some("字典项不存在".into()),
                )
            })?;
        self.check_item_value_unique(
            existing.dict_code.as_deref().unwrap_or_default(),
            &f.value,
            Some(f.id),
        )
        .await?;

        let mut a: entity::sys_dict_item::ActiveModel = existing.into();
        a.value = Set(Some(f.value.clone()));
        a.label = Set(Some(f.label.clone()));
        a.tag_type = Set(f.tag_type.clone());
        a.status = Set(f.status.unwrap_or(1));
        a.sort = Set(f.sort.unwrap_or(0));
        a.update(self.db).await?;
        Ok(())
    }

    /// 批量删除字典项（逻辑删除）
    pub async fn delete_items(&self, ids: &str) -> AppResult<u64> {
        let ids: Vec<i64> = ids
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        Ok(entity::sys_dict_item::Entity::update_many()
            .col_expr(entity::sys_dict_item::Column::IsDeleted, Expr::value(1))
            .filter(entity::sys_dict_item::Column::Id.is_in(ids))
            .filter(entity::sys_dict_item::Column::IsDeleted.eq(0))
            .exec(self.db)
            .await?
            .rows_affected)
    }

    /// 校验同一字典下字典项值唯一
    async fn check_item_value_unique(
        &self,
        dict_code: &str,
        value: &str,
        exclude_id: Option<i64>,
    ) -> AppResult<()> {
        let mut select = entity::sys_dict_item::Entity::find()
            .filter(entity::sys_dict_item::Column::DictCode.eq(dict_code))
            .filter(entity::sys_dict_item::Column::Value.eq(value))
            .filter(entity::sys_dict_item::Column::IsDeleted.eq(0));
        if let Some(id) = exclude_id {
            select = select.filter(entity::sys_dict_item::Column::Id.ne(id));
        }
        if select.one(self.db).await?.is_some() {
            return Err(AppError::Business(
                ResultCode::IntegrityConstraintViolation,
                Some("该字典下字典项值已存在".into()),
            ));
        }
        Ok(())
    }

    /// 删除前查询字典编码列表
    pub async fn get_codes_by_ids(&self, ids: &str) -> AppResult<Vec<String>> {
        let ids: Vec<i64> = ids
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let rows = entity::sys_dict::Entity::find()
            .filter(entity::sys_dict::Column::Id.is_in(ids))
            .filter(entity::sys_dict::Column::IsDeleted.eq(0))
            .all(self.db)
            .await?;
        Ok(rows.into_iter().filter_map(|r| r.dict_code).collect())
    }
}
