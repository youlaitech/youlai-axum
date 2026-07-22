use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_dict")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub dict_code: Option<String>,
    pub name: Option<String>,
    #[sea_orm(default_value = 0)]
    pub status: i16,
    pub remark: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub create_by: Option<i64>,
    pub update_time: Option<NaiveDateTime>,
    pub update_by: Option<i64>,
    #[sea_orm(default_value = 0)]
    pub is_deleted: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sys_dict_item::Entity")]
    SysDictItem,
}

impl Related<super::sys_dict_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysDictItem.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
