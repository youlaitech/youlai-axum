use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_menu")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub parent_id: i64,
    pub tree_path: Option<String>,
    pub name: String,
    #[sea_orm(column_type = "Char(Some(1))")]
    pub r#type: String,
    pub route_name: Option<String>,
    pub route_path: Option<String>,
    pub component: Option<String>,
    pub perm: Option<String>,
    pub external_url: Option<String>,
    #[sea_orm(default_value = 0)]
    pub always_show: Option<i16>,
    #[sea_orm(default_value = 0)]
    pub keep_alive: Option<i16>,
    #[sea_orm(default_value = 1)]
    pub visible: Option<i16>,
    #[sea_orm(default_value = 0)]
    pub sort: Option<i32>,
    pub icon: Option<String>,
    pub redirect: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub update_time: Option<NaiveDateTime>,
    pub params: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sys_role_menu::Entity")]
    SysRoleMenu,
}

impl Related<super::sys_role_menu::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SysRoleMenu.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
