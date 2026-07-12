//! 部门管理服务：树形 CRUD，删除前校验子部门和用户

use sea_orm::*;
use sea_orm::sea_query::Expr;
use crate::common::enums::ResultCode;
use crate::entity;
use crate::framework::web::error::{AppError, AppResult};
use crate::modules::system::dept::dto::*;

pub struct DeptService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> DeptService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    /// 获取部门树
    pub async fn get_tree(&self) -> AppResult<Vec<DeptVO>> {
        let all = entity::sys_dept::Entity::find()
            .filter(entity::sys_dept::Column::IsDeleted.eq(0))
            .order_by_asc(entity::sys_dept::Column::Sort)
            .all(self.db).await?;
        Ok(build_tree(&all, 0))
    }

    /// 获取部门表单数据
    pub async fn get_form(&self, id: i64) -> AppResult<DeptVO> {
        let d = entity::sys_dept::Entity::find_by_id(id).one(self.db).await?.unwrap();
        Ok(entity_to_vo(&d))
    }

    /// 新增部门（自动计算 tree_path 实现无限层级）
    pub async fn create(&self, f: &DeptCreate) -> AppResult<i64> {
        // tree_path 规则：顶级="0"，子级="父级tree_path,父级id"
        let tree_path = if f.parent_id == 0 { "0".to_string() } else {
            let parent = entity::sys_dept::Entity::find_by_id(f.parent_id).one(self.db).await?.unwrap();
            format!("{},{}", parent.tree_path, f.parent_id)
        };
        let a = entity::sys_dept::ActiveModel {
            parent_id: Set(f.parent_id), name: Set(f.name.clone()), code: Set(f.code.clone()),
            sort: Set(f.sort.unwrap_or(0)), status: Set(f.status.unwrap_or(1)),
            tree_path: Set(tree_path), ..Default::default()
        };
        Ok(a.insert(self.db).await?.id)
    }

    /// 更新部门
    pub async fn update(&self, f: &DeptUpdate) -> AppResult<()> {
        let mut a: entity::sys_dept::ActiveModel = entity::sys_dept::Entity::find_by_id(f.id)
            .one(self.db).await?.unwrap().into();
        a.parent_id = Set(f.parent_id); a.name = Set(f.name.clone()); a.code = Set(f.code.clone());
        a.sort = Set(f.sort.unwrap_or(0)); a.status = Set(f.status.unwrap_or(1));
        a.update(self.db).await?;
        Ok(())
    }

    /// 批量删除部门（逻辑删除），检查子部门和关联用户
    pub async fn delete(&self, ids: &str) -> AppResult<u64> {
        let id_list: Vec<i64> = ids.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if id_list.is_empty() { return Err(AppError::Business(ResultCode::UserRequestParameterError, None)); }
        // 逐个检查是否存在子部门或关联用户
        for id in &id_list {
            let children = entity::sys_dept::Entity::find()
                .filter(entity::sys_dept::Column::ParentId.eq(*id))
                .filter(entity::sys_dept::Column::IsDeleted.eq(0)).count(self.db).await?;
            if children > 0 { return Err(AppError::Business(ResultCode::AccessPermissionException, Some("存在子部门，无法删除".into()))); }
            let users = entity::sys_user::Entity::find()
                .filter(entity::sys_user::Column::DeptId.eq(*id))
                .filter(entity::sys_user::Column::IsDeleted.eq(0)).count(self.db).await?;
            if users > 0 { return Err(AppError::Business(ResultCode::AccessPermissionException, Some("部门存在用户，无法删除".into()))); }
        }
        Ok(entity::sys_dept::Entity::update_many()
            .col_expr(entity::sys_dept::Column::IsDeleted, Expr::value(1))
            .filter(entity::sys_dept::Column::Id.is_in(id_list)).exec(self.db).await?.rows_affected)
    }

    /// 部门下拉选项
    pub async fn get_options(&self) -> AppResult<Vec<DeptOptionVO>> {
        let all = entity::sys_dept::Entity::find()
            .filter(entity::sys_dept::Column::IsDeleted.eq(0))
            .filter(entity::sys_dept::Column::Status.eq(1))
            .order_by_asc(entity::sys_dept::Column::Sort).all(self.db).await?;
        Ok(build_option_tree(&all, 0))
    }
}

fn build_tree(all: &[entity::sys_dept::Model], pid: i64) -> Vec<DeptVO> {
    all.iter().filter(|d| d.parent_id == pid).map(|d| DeptVO {
        id: d.id, name: d.name.clone(), code: d.code.clone(), parent_id: d.parent_id,
        tree_path: d.tree_path.clone(), sort: Some(d.sort), status: d.status,
        create_time: d.create_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        children: build_tree(all, d.id),
    }).collect()
}

fn entity_to_vo(d: &entity::sys_dept::Model) -> DeptVO {
    DeptVO { id: d.id, name: d.name.clone(), code: d.code.clone(), parent_id: d.parent_id,
        tree_path: d.tree_path.clone(), sort: Some(d.sort), status: d.status,
        create_time: d.create_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()), children: vec![] }
}

fn build_option_tree(all: &[entity::sys_dept::Model], pid: i64) -> Vec<DeptOptionVO> {
    all.iter().filter(|d| d.parent_id == pid).map(|d| DeptOptionVO {
        value: d.id, label: d.name.clone(), children: build_option_tree(all, d.id),
    }).collect()
}
