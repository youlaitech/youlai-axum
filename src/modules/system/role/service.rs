//! 角色管理服务

use crate::entity;
use crate::error::{AppError, AppResult};
use crate::modules::system::role::dto::*;
use crate::pagination::PageData;
use crate::result_code::ResultCode;
use sea_orm::sea_query::Expr;
use sea_orm::*;

pub struct RoleService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> RoleService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 角色分页列表
    pub async fn get_page(&self, q: &RoleQuery) -> AppResult<PageData<RoleVO>> {
        let mut s =
            entity::sys_role::Entity::find().filter(entity::sys_role::Column::IsDeleted.eq(0));
        if let Some(ref kw) = q.keywords {
            if !kw.is_empty() {
                s = s.filter(entity::sys_role::Column::Name.like(format!("%{kw}%")));
            }
        }
        let total = s.clone().count(self.db).await?;
        let rows = s
            .order_by_asc(entity::sys_role::Column::Sort)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;
        Ok(PageData::new(
            rows.iter()
                .map(|r| RoleVO {
                    id: r.id,
                    name: r.name.clone(),
                    code: r.code.clone(),
                    sort: r.sort,
                    status: r.status,
                    data_scope: r.data_scope,
                    create_time: r
                        .create_time
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                    update_time: r
                        .update_time
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                    menu_ids: None,
                })
                .collect(),
            total,
        ))
    }

    /// 获取角色表单
    pub async fn get_form(&self, id: i64) -> AppResult<RoleVO> {
        let r = entity::sys_role::Entity::find_by_id(id)
            .filter(entity::sys_role::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| AppError::Business(ResultCode::AccountNotFound, None))?;
        Ok(RoleVO {
            id: r.id,
            name: r.name,
            code: r.code,
            sort: r.sort,
            status: r.status,
            data_scope: r.data_scope,
            create_time: r
                .create_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            update_time: r
                .update_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            menu_ids: None,
        })
    }

    /// 新增角色
    pub async fn create(&self, f: &RoleCreate) -> AppResult<i64> {
        let now = chrono::Local::now().naive_local();
        Ok(entity::sys_role::ActiveModel {
            name: Set(f.name.clone()),
            code: Set(f.code.clone()),
            sort: Set(f.sort),
            status: Set(f.status.unwrap_or(1)),
            data_scope: Set(f.data_scope),
            create_time: Set(Some(now)),
            update_time: Set(Some(now)),
            ..Default::default()
        }
        .insert(self.db)
        .await?
        .id)
    }

    /// 更新角色
    pub async fn update(&self, f: &RoleUpdate) -> AppResult<()> {
        let mut a: entity::sys_role::ActiveModel = entity::sys_role::Entity::find_by_id(f.id)
            .one(self.db)
            .await?
            .unwrap()
            .into();
        a.name = Set(f.name.clone());
        a.code = Set(f.code.clone());
        a.sort = Set(f.sort);
        a.status = Set(f.status.unwrap_or(1));
        a.data_scope = Set(f.data_scope);
        a.update(self.db).await?;
        Ok(())
    }

    /// 更新角色状态
    pub async fn update_status(&self, id: i64, status: i16) -> AppResult<()> {
        entity::sys_role::Entity::update_many()
            .col_expr(entity::sys_role::Column::Status, Expr::value(status))
            .filter(entity::sys_role::Column::Id.eq(id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 批量删除角色（逻辑删除），级联清理关联表
    pub async fn delete(&self, ids: &str) -> AppResult<u64> {
        let ids: Vec<i64> = ids
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        // 逻辑删除角色
        let r = entity::sys_role::Entity::update_many()
            .col_expr(entity::sys_role::Column::IsDeleted, Expr::value(1))
            .filter(entity::sys_role::Column::Id.is_in(ids.clone()))
            .exec(self.db)
            .await?;
        // 级联清理：角色-菜单、角色-部门关联表
        entity::sys_role_menu::Entity::delete_many()
            .filter(entity::sys_role_menu::Column::RoleId.is_in(ids.clone()))
            .exec(self.db)
            .await?;
        entity::sys_role_dept::Entity::delete_many()
            .filter(entity::sys_role_dept::Column::RoleId.is_in(ids.clone()))
            .exec(self.db)
            .await?;
        Ok(r.rows_affected)
    }

    /// 为角色分配菜单
    pub async fn assign_menus(&self, id: i64, menu_ids: &[i64]) -> AppResult<()> {
        entity::sys_role_menu::Entity::delete_many()
            .filter(entity::sys_role_menu::Column::RoleId.eq(id))
            .exec(self.db)
            .await?;
        for &mid in menu_ids {
            entity::sys_role_menu::ActiveModel {
                role_id: Set(id),
                menu_id: Set(mid),
            }
            .insert(self.db)
            .await?;
        }
        Ok(())
    }

    /// 获取角色关联的菜单 ID 列表
    pub async fn get_menu_ids(&self, id: i64) -> AppResult<Vec<i64>> {
        Ok(entity::sys_role_menu::Entity::find()
            .filter(entity::sys_role_menu::Column::RoleId.eq(id))
            .all(self.db)
            .await?
            .iter()
            .map(|m| m.menu_id)
            .collect())
    }

    /// 获取角色关联的部门 ID 列表（数据权限）
    pub async fn get_dept_ids(&self, id: i64) -> AppResult<Vec<i64>> {
        Ok(entity::sys_role_dept::Entity::find()
            .filter(entity::sys_role_dept::Column::RoleId.eq(id))
            .all(self.db)
            .await?
            .iter()
            .map(|d| d.dept_id)
            .collect())
    }

    /// 更新角色数据权限范围及自定义部门
    pub async fn update_data_scope(&self, id: i64, f: &DataScopeForm) -> AppResult<()> {
        entity::sys_role::Entity::update_many()
            .col_expr(
                entity::sys_role::Column::DataScope,
                Expr::value(f.data_scope),
            )
            .filter(entity::sys_role::Column::Id.eq(id))
            .exec(self.db)
            .await?;
        if let Some(ref dept_ids) = f.dept_ids {
            entity::sys_role_dept::Entity::delete_many()
                .filter(entity::sys_role_dept::Column::RoleId.eq(id))
                .exec(self.db)
                .await?;
            for &did in dept_ids {
                entity::sys_role_dept::ActiveModel {
                    role_id: Set(id),
                    dept_id: Set(did),
                }
                .insert(self.db)
                .await?;
            }
        }
        Ok(())
    }

    /// 角色下拉选项
    pub async fn get_options(&self) -> AppResult<Vec<RoleOptionVO>> {
        Ok(entity::sys_role::Entity::find()
            .filter(entity::sys_role::Column::IsDeleted.eq(0))
            .filter(entity::sys_role::Column::Status.eq(1))
            .all(self.db)
            .await?
            .iter()
            .map(|r| RoleOptionVO {
                value: r.id,
                label: r.name.clone(),
            })
            .collect())
    }
}
