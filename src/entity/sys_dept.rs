use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_dept")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub code: String,
    #[sea_orm(default_value = 0)]
    pub parent_id: i64,
    pub tree_path: String,
    #[sea_orm(default_value = 0)]
    pub sort: i16,
    #[sea_orm(default_value = 1)]
    pub status: i16,
    pub create_by: Option<i64>,
    pub create_time: Option<NaiveDateTime>,
    pub update_by: Option<i64>,
    pub update_time: Option<NaiveDateTime>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
