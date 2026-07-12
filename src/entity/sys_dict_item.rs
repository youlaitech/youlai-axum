use sea_orm::entity::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_dict_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub dict_code: Option<String>,
    pub value: Option<String>,
    pub label: Option<String>,
    pub tag_type: Option<String>,
    #[sea_orm(default_value = 0)]
    pub status: i16,
    #[sea_orm(default_value = 0)]
    pub sort: i32,
    pub remark: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub create_by: Option<i64>,
    pub update_time: Option<NaiveDateTime>,
    pub update_by: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::sys_dict::Entity", from = "Column::DictCode", to = "super::sys_dict::Column::DictCode")]
    SysDict,
}

impl Related<super::sys_dict::Entity> for Entity { fn to() -> RelationDef { Relation::SysDict.def() } }

impl ActiveModelBehavior for ActiveModel {}
