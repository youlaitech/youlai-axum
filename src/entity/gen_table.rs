#![allow(dead_code)]
use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gen_table")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub table_name: String,
    pub module_name: Option<String>,
    pub package_name: String,
    pub business_name: String,
    pub entity_name: String,
    pub author: String,
    pub parent_menu_id: Option<i64>,
    pub remove_table_prefix: Option<String>,
    pub page_type: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_time: Option<NaiveDateTime>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
