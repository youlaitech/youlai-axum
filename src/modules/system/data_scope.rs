//! 数据权限过滤引擎：ALL（放行）/ DEPT_AND_SUB / DEPT / SELF / CUSTOM

use crate::entity;
use crate::modules::auth::principal::Principal;
use sea_orm::*;

/// 对 SeaORM Select 追加数据权限过滤条件。
/// 超级管理员或拥有 ALL(1) 权限角色时直接返回原 Select。
pub async fn apply_data_scope<E: EntityTrait>(
    db: &DatabaseConnection,
    select: Select<E>,
    user: &Principal,
    dept_col: E::Column,
    creator_col: E::Column,
) -> Select<E>
where
    E::Column: ColumnTrait + Copy,
{
    // 超级管理员 / ALL 权限 — 跳过过滤
    if user.is_root || user.data_scopes.iter().any(|s| s.data_scope == 1) {
        return select;
    }

    // 多角色数据权限取并集（OR）
    let mut conditions = Condition::any();

    for scope in &user.data_scopes {
        match scope.data_scope {
            // DEPT_AND_SUB(2)：本部门及所有子部门
            2 => {
                if let Some(did) = user.dept_id {
                    let sub_ids = get_dept_and_sub_ids(db, did).await;
                    conditions = conditions.add(dept_col.is_in(sub_ids));
                }
            }
            // DEPT(3)：仅本部门
            3 => {
                if let Some(did) = user.dept_id {
                    conditions = conditions.add(dept_col.eq(did));
                }
            }
            // SELF(4)：仅本人数据（create_by = user_id）
            4 => {
                conditions = conditions.add(creator_col.eq(user.user_id));
            }
            // CUSTOM(5)：指定部门列表
            5 => {
                if let Some(ref ids) = scope.dept_ids {
                    if !ids.is_empty() {
                        conditions = conditions.add(dept_col.is_in(ids.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    if !conditions.is_empty() {
        select.filter(conditions)
    } else {
        select
    }
}

/// 获取部门及其所有子部门 ID 列表，通过 tree_path 前缀匹配
async fn get_dept_and_sub_ids(db: &DatabaseConnection, dept_id: i64) -> Vec<i64> {
    let dept = entity::sys_dept::Entity::find_by_id(dept_id)
        .filter(entity::sys_dept::Column::IsDeleted.eq(0))
        .one(db)
        .await;
    let Some(dept) = dept.ok().flatten() else {
        return vec![dept_id];
    };

    // tree_path 格式："0,1"，匹配自身和所有子树节点
    let tree_path = format!("{},{}", dept.tree_path, dept_id);
    entity::sys_dept::Entity::find()
        .filter(entity::sys_dept::Column::IsDeleted.eq(0))
        .filter(
            Condition::any()
                .add(entity::sys_dept::Column::Id.eq(dept_id))
                .add(entity::sys_dept::Column::TreePath.like(format!("{}%", &tree_path))),
        )
        .all(db)
        .await
        .map(|list| list.iter().map(|d| d.id).collect())
        .unwrap_or_else(|_| vec![dept_id])
}
