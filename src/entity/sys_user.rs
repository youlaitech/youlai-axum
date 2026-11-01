use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub username: Option<String>,
    pub nickname: Option<String>,
    #[sea_orm(default_value = 1)]
    pub gender: Option<i16>,
    pub password: Option<String>,
    pub dept_id: Option<i64>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
    #[sea_orm(default_value = 1)]
    pub status: Option<i16>,
    pub email: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub create_by: Option<i64>,
    pub update_time: Option<NaiveDateTime>,
    pub update_by: Option<i64>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sys_user_role::Entity")]
    SysUserRole,
    #[sea_orm(has_many = "super::sys_user_identity::Entity")]
    SysUserIdentity,
}

impl Related<super::sys_user_role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysUserRole.def()
    }
}
impl Related<super::sys_user_identity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysUserIdentity.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
