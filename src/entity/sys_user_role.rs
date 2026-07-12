use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_user_role")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: i64,
    #[sea_orm(primary_key)]
    pub role_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::sys_role::Entity", from = "Column::RoleId", to = "super::sys_role::Column::Id")]
    SysRole,
    #[sea_orm(belongs_to = "super::sys_user::Entity", from = "Column::UserId", to = "super::sys_user::Column::Id")]
    SysUser,
}

impl Related<super::sys_role::Entity> for Entity { fn to() -> RelationDef { Relation::SysRole.def() } }
impl Related<super::sys_user::Entity> for Entity { fn to() -> RelationDef { Relation::SysUser.def() } }

impl ActiveModelBehavior for ActiveModel {}
