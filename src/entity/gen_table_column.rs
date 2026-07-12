#![allow(dead_code)]
use sea_orm::entity::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gen_table_column")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub table_id: i64,
    pub column_name: Option<String>,
    pub column_type: Option<String>,
    pub column_length: Option<i32>,
    pub field_name: String,
    pub field_type: Option<String>,
    pub field_sort: Option<i32>,
    pub field_comment: Option<String>,
    pub max_length: Option<i32>,
    pub is_required: Option<i16>,
    #[sea_orm(default_value = 0)]
    pub is_show_in_list: i16,
    #[sea_orm(default_value = 0)]
    pub is_show_in_form: i16,
    #[sea_orm(default_value = 0)]
    pub is_show_in_query: i16,
    pub query_type: Option<i16>,
    pub form_type: Option<i16>,
    pub dict_type: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_time: Option<NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
