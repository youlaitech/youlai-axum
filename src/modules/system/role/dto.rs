use crate::pagination::{default_page_num, default_page_size};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub keywords: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RoleCreate {
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    #[validate(length(min = 1, max = 32))]
    pub code: String,
    pub sort: Option<i32>,
    pub status: Option<i16>,
    pub data_scope: Option<i16>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RoleUpdate {
    pub id: i64,
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    #[validate(length(min = 1, max = 32))]
    pub code: String,
    pub sort: Option<i32>,
    pub status: Option<i16>,
    pub data_scope: Option<i16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleStatusQuery {
    pub status: i16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataScopeForm {
    pub data_scope: i16,
    #[serde(rename = "deptIds")]
    pub dept_ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleVO {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub sort: Option<i32>,
    pub status: i16,
    pub data_scope: Option<i16>,
    pub create_time: Option<String>,
    pub menu_ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleOptionVO {
    pub value: i64,
    pub label: String,
}
