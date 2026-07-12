use serde::{Deserialize, Serialize};

/// 统一响应结构体 `{code, msg, data}`
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Result<T: Serialize> {
    pub code: String,
    pub msg: String,
    pub data: Option<T>,
}

impl<T: Serialize> Result<T> {
    /// 成功响应，附带数据
    pub fn success(data: T) -> Self {
        Self { code: "00000".into(), msg: "成功".into(), data: Some(data) }
    }

    /// 成功响应，无数据
    pub fn success_empty() -> Self {
        Self { code: "00000".into(), msg: "成功".into(), data: None }
    }

    /// 失败响应
    pub fn failed(code: crate::common::enums::ResultCode, msg: Option<String>) -> Self {
        Self { code: code.code().into(), msg: msg.unwrap_or_else(|| code.msg().into()), data: None }
    }
}

/// 分页响应结构体 `{code, msg, data: {list, total}}`
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T: Serialize> {
    pub code: String,
    pub msg: String,
    pub data: PageData<T>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData<T: Serialize> {
    pub list: Vec<T>,
    pub total: u64,
}

impl<T: Serialize> PageResult<T> {
    pub fn success(list: Vec<T>, total: u64) -> Self {
        Self { code: "00000".into(), msg: "成功".into(), data: PageData { list, total } }
    }
}
