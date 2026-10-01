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
    #[serde(default, deserialize_with = "crate::util::opt_i64_from_any")]
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
    #[serde(default, deserialize_with = "crate::util::opt_i64_from_any")]
    pub parent_id: Option<i64>,
    #[serde(default, deserialize_with = "crate::util::opt_vec_i64_from_any")]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 复现接口测试「修改表单定义」的入参：id 为列表回传的字符串
    #[test]
    fn definition_form_accepts_string_id() {
        let body = r#"{
            "id": "1004",
            "formKey": "apitest_form_x",
            "formName": "接口测试表单改_X",
            "description": "apitest",
            "formJson": "[{\"type\":\"input\",\"field\":\"name\",\"title\":\"姓名\"}]",
            "optionsJson": "{}",
            "isPublic": 1,
            "category": "normal"
        }"#;
        let f: FormDefinitionForm = serde_json::from_str(body).expect("字符串 id 应可解析");
        assert_eq!(f.id, Some(1004));
        assert_eq!(f.form_key, "apitest_form_x");
    }

    /// 新增时不传 id，也应正常解析
    #[test]
    fn definition_form_allows_missing_id() {
        let body = r#"{"formKey":"k","formName":"n"}"#;
        let f: FormDefinitionForm = serde_json::from_str(body).expect("缺省 id 应可解析");
        assert_eq!(f.id, None);
    }

    /// 发布向导：菜单树回传的 parentId 与角色多选回传的 roleIds 均为字符串
    #[test]
    fn menu_form_accepts_string_ids() {
        let body = r#"{"menuName":"菜单","parentId":"0","roleIds":["1","2"]}"#;
        let f: FormMenuForm = serde_json::from_str(body).expect("字符串 ID 应可解析");
        assert_eq!(f.parent_id, Some(0));
        assert_eq!(f.role_ids, Some(vec![1, 2]));
    }
}
