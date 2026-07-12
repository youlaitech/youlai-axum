use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::common::pagination::{default_page_num, default_page_size};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub keywords: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCreate {
    #[validate(length(min = 1, max = 50))]
    pub config_name: String,
    #[validate(length(min = 1, max = 50))]
    pub config_key: String,
    #[validate(length(min = 1, max = 100))]
    pub config_value: String,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdate {
    pub id: i64,
    pub config_name: String,
    pub config_key: String,
    pub config_value: String,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigVO {
    pub id: i64,
    pub config_name: String,
    pub config_key: String,
    pub config_value: String,
    pub remark: Option<String>,
    pub create_time: Option<String>,
}
