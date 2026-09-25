use crate::pagination::{default_page_num, default_page_size};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormDefinitionQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub keywords: Option<String>,
    pub status: Option<i16>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormDataQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FormDefinitionForm {
    pub id: Option<i64>,
    #[validate(length(min = 1, max = 64))]
    pub form_key: String,
    #[validate(length(min = 1, max = 100))]
    pub form_name: String,
    pub description: Option<String>,
    pub form_json: Option<String>,
    pub options_json: Option<String>,
    pub is_public: Option<i16>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FormMenuForm {
    #[validate(length(min = 1, max = 50))]
    pub menu_name: String,
    pub parent_id: Option<i64>,
    pub role_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FormAiGenerateForm {
    #[validate(length(min = 1))]
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormDefinitionVO {
    #[serde(serialize_with = "crate::util::i64_to_string")]
    pub id: i64,
    pub form_key: String,
    pub form_name: String,
    pub description: Option<String>,
    pub status: i16,
    pub version: i32,
    pub is_public: i16,
    pub category: String,
    pub create_time: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormDataVO {
    #[serde(serialize_with = "crate::util::i64_to_string")]
    pub id: i64,
    pub form_version: i32,
    pub data_json: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_json: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options_json: Option<Value>,
    pub create_by: Option<String>,
    pub create_by_name: Option<String>,
    pub create_time: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormMenuVO {
    #[serde(serialize_with = "crate::util::i64_to_string")]
    pub menu_id: i64,
    pub menu_name: String,
    #[serde(serialize_with = "crate::util::i64_to_string")]
    pub parent_id: i64,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormRenderVO {
    pub form_key: String,
    pub form_name: String,
    pub version: i32,
    pub form_json: Value,
    pub options_json: Option<Value>,
}
