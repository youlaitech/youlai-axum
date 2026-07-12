use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::common::pagination::{default_page_num, default_page_size};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub keywords: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DictCreate {
    #[validate(length(min = 1, max = 50))]
    pub dict_code: String,
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    pub status: Option<i16>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DictUpdate {
    pub id: i64,
    #[validate(length(min = 1, max = 50))]
    pub dict_code: String,
    pub name: String,
    pub status: Option<i16>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictItemQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DictItemCreate {
    pub value: String,
    pub label: String,
    pub tag_type: Option<String>,
    pub status: Option<i16>,
    pub sort: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DictItemUpdate {
    pub id: i64,
    pub value: String,
    pub label: String,
    pub tag_type: Option<String>,
    pub status: Option<i16>,
    pub sort: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictVO {
    pub id: i64,
    pub dict_code: Option<String>,
    pub name: Option<String>,
    pub status: i16,
    pub remark: Option<String>,
    pub create_time: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictItemVO {
    pub id: i64,
    pub dict_code: Option<String>,
    pub value: Option<String>,
    pub label: Option<String>,
    pub tag_type: Option<String>,
    pub status: i16,
    pub sort: i32,
}
