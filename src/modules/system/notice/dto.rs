use crate::pagination::{default_page_num, default_page_size};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub title: Option<String>,
    pub publish_status: Option<i16>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct NoticeCreate {
    #[validate(length(min = 1, max = 50))]
    pub title: String,
    pub content: Option<String>,
    pub r#type: i16,
    pub level: String,
    pub target_type: i16,
    pub target_user_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct NoticeUpdate {
    pub id: i64,
    pub title: String,
    pub content: Option<String>,
    pub r#type: i16,
    pub level: String,
    pub target_type: i16,
    pub target_user_ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeVO {
    pub id: i64,
    pub title: Option<String>,
    pub content: Option<String>,
    pub r#type: i16,
    pub level: String,
    pub target_type: i16,
    pub target_user_ids: Option<String>,
    pub publish_status: i16,
    pub publisher_id: Option<i64>,
    pub publisher_name: Option<String>,
    pub publish_time: Option<String>,
    pub revoke_time: Option<String>,
    pub create_time: Option<String>,
    pub is_read: Option<i16>,
}
