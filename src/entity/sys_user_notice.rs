use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_user_notice")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub notice_id: i64,
    pub user_id: i64,
    #[sea_orm(default_value = 0)]
    pub is_read: i16,
    pub read_time: Option<NaiveDateTime>,
    pub create_time: NaiveDateTime,
    pub update_time: Option<NaiveDateTime>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
