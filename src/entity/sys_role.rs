use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_role")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub code: String,
    pub sort: Option<i32>,
    #[sea_orm(default_value = 1)]
    pub status: i16,
    pub data_scope: Option<i16>,
    pub create_by: Option<i64>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<i64>,
    pub update_time: Option<NaiveDateTime>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sys_user_role::Entity")]
    SysUserRole,
    #[sea_orm(has_many = "super::sys_role_menu::Entity")]
    SysRoleMenu,
    #[sea_orm(has_many = "super::sys_role_dept::Entity")]
    SysRoleDept,
}

impl Related<super::sys_user_role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysUserRole.def()
    }
}
impl Related<super::sys_role_menu::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysRoleMenu.def()
    }
}
impl Related<super::sys_role_dept::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysRoleDept.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
