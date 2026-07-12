use sea_orm::entity::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub module: i16,
    pub action_type: i16,
    pub title: String,
    pub content: Option<String>,
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub request_uri: Option<String>,
    pub request_method: Option<String>,
    pub ip: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub device: Option<String>,
    pub os: Option<String>,
    pub browser: Option<String>,
    #[sea_orm(default_value = 1)]
    pub status: i16,
    pub error_msg: Option<String>,
    pub execution_time: Option<i32>,
    pub create_time: Option<NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
