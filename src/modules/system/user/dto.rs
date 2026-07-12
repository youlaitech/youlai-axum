use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::common::pagination::{default_page_num, default_page_size};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub keywords: Option<String>,
    pub status: Option<i16>,
    pub dept_id: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserCreate {
    #[validate(length(min = 3, max = 64))]
    pub username: String,
    #[validate(length(min = 1, max = 64))]
    pub nickname: String,
    #[serde(default = "default_gender")]
    pub gender: i16,
    pub dept_id: Option<i64>,
    #[validate(length(min = 11, max = 11))]
    pub mobile: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[serde(default = "default_status")]
    pub status: i16,
    pub role_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserUpdate {
    pub id: i64,
    #[validate(length(min = 1, max = 64))]
    pub nickname: String,
    #[serde(default = "default_gender")]
    pub gender: i16,
    pub dept_id: Option<i64>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    #[serde(default = "default_status")]
    pub status: i16,
    pub role_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStatusQuery {
    pub status: i16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPasswordQuery {
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileUpdate {
    pub nickname: Option<String>,
    pub gender: Option<i16>,
    pub mobile: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordForm {
    #[serde(rename = "oldPassword")]
    pub old_password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCodeForm {
    pub mobile: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindMobileForm {
    pub mobile: String,
    pub code: String,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailCodeForm {
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct BindEmailForm {
    pub email: String,
    pub code: String,
    #[serde(default)]
    pub password: Option<String>,
}

fn default_gender() -> i16 {
    1
}
fn default_status() -> i16 {
    1
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserVO {
    pub id: i64,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub gender: i16,
    pub avatar: Option<String>,
    pub dept_id: Option<i64>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub status: i16,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
    pub role_ids: Option<Vec<i64>>,
    pub dept_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileVO {
    pub id: i64,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub gender: i16,
    pub avatar: Option<String>,
    pub dept_id: Option<i64>,
    pub dept_name: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub perms: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOptionVO {
    pub value: i64,
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelResultVO {
    #[serde(rename = "validCount")]
    pub valid_count: u32,
    #[serde(rename = "invalidCount")]
    pub invalid_count: u32,
    #[serde(rename = "messageList")]
    pub message_list: Vec<String>,
}
