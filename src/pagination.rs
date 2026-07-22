#![allow(dead_code)]
use serde::{Deserialize, Serialize};

/// 分页查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageQuery {
    #[serde(default = "default_page_num")]
    pub page_num: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

pub fn default_page_num() -> i64 {
    1
}
pub fn default_page_size() -> i64 {
    10
}

/// 分页响应数据
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData<T: Serialize> {
    pub list: Vec<T>,
    pub total: u64,
}

impl<T: Serialize> PageData<T> {
    pub fn new(list: Vec<T>, total: u64) -> Self {
        Self { list, total }
    }
}
