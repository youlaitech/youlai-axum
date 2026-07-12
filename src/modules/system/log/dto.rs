use serde::{Deserialize, Serialize};
use crate::common::pagination::{default_page_num, default_page_size};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub module: Option<i16>,
    pub action_type: Option<i16>,
    pub status: Option<i16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogVO {
    pub id: i64,
    pub module: i16,
    pub action_type: i16,
    pub title: String,
    pub content: Option<String>,
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub request_uri: Option<String>,
    pub request_method: Option<String>,
    pub ip: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub device: Option<String>,
    pub os: Option<String>,
    pub browser: Option<String>,
    pub status: i16,
    pub error_msg: Option<String>,
    pub execution_time: Option<i32>,
    pub create_time: Option<String>,
}
