use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "form_snapshot")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub form_id: i64,
    pub version: i32,
    pub form_json: Json,
    pub options_json: Option<Json>,
    pub create_time: Option<NaiveDateTime>,
    pub update_time: Option<NaiveDateTime>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
