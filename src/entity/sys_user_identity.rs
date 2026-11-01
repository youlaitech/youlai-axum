use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_user_identity")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub last_login_time: Option<NaiveDateTime>,
    pub create_time: Option<NaiveDateTime>,
    pub update_time: Option<NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sys_user::Entity",
        from = "Column::UserId",
        to = "super::sys_user::Column::Id"
    )]
    SysUser,
}

impl Related<super::sys_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
