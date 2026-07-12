use sea_orm::entity::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_notice")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub title: Option<String>,
    pub content: Option<String>,
    pub r#type: i16,
    pub level: String,
    pub target_type: i16,
    pub target_user_ids: Option<String>,
    pub publisher_id: Option<i64>,
    #[sea_orm(default_value = 0)]
    pub publish_status: i16,
    pub publish_time: Option<NaiveDateTime>,
    pub revoke_time: Option<NaiveDateTime>,
    pub create_by: i64,
    pub create_time: NaiveDateTime,
    pub update_by: Option<i64>,
    pub update_time: Option<NaiveDateTime>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
