use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub config_name: String,
    pub config_key: String,
    pub config_value: String,
    pub remark: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub create_by: Option<i64>,
    pub update_time: Option<NaiveDateTime>,
    pub update_by: Option<i64>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
