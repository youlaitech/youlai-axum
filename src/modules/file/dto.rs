//! 文件模块的请求体与响应 VO。

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileQuery {
    pub file_path: String,
}
