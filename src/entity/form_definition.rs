use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "form_definition")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub form_key: String,
    pub form_name: String,
    pub description: Option<String>,
    pub form_json: Json,
    pub options_json: Option<Json>,
    #[sea_orm(default_value = 0)]
    pub status: i16,
    #[sea_orm(default_value = 0)]
    pub is_public: i16,
    #[sea_orm(default_value = "normal")]
    pub category: String,
    pub menu_id: Option<i64>,
    #[sea_orm(default_value = 1)]
    pub version: i32,
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
