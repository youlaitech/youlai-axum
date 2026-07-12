use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeptCreate {
    pub parent_id: i64,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 1, max = 100))]
    pub code: String,
    pub sort: Option<i16>,
    pub status: Option<i16>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeptUpdate {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub code: String,
    pub sort: Option<i16>,
    pub status: Option<i16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeptVO {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub parent_id: i64,
    pub tree_path: String,
    pub sort: Option<i16>,
    pub status: i16,
    pub create_time: Option<String>,
    pub children: Vec<DeptVO>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeptOptionVO {
    pub value: i64,
    pub label: String,
    pub children: Vec<DeptOptionVO>,
}
