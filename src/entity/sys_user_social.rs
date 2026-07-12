use sea_orm::entity::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_user_social")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    pub platform: String,
    pub openid: String,
    pub unionid: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub session_key: Option<String>,
    #[sea_orm(default_value = 1)]
    pub verified: i16,
    pub create_time: Option<NaiveDateTime>,
    pub update_time: Option<NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::sys_user::Entity", from = "Column::UserId", to = "super::sys_user::Column::Id")]
    SysUser,
}

impl Related<super::sys_user::Entity> for Entity { fn to() -> RelationDef { Relation::SysUser.def() } }

impl ActiveModelBehavior for ActiveModel {}
