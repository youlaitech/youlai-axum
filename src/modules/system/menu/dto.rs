use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MenuCreate {
    pub parent_id: i64,
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    #[validate(length(min = 1, max = 1))]
    pub r#type: String,
    pub route_name: Option<String>,
    pub route_path: Option<String>,
    pub component: Option<String>,
    pub external_url: Option<String>,
    pub perm: Option<String>,
    pub visible: Option<i16>,
    pub sort: Option<i32>,
    pub icon: Option<String>,
    pub redirect: Option<String>,
    pub keep_alive: Option<i16>,
    /// 新增页面菜单时是否生成增删改查按钮
    #[serde(default)]
    pub generate_crud_buttons: Option<bool>,
    /// 按钮权限标识前缀，如 sys:user
    pub button_perm_prefix: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MenuUpdate {
    pub id: i64,
    pub parent_id: i64,
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub r#type: String,
    pub route_name: Option<String>,
    pub route_path: Option<String>,
    pub component: Option<String>,
    pub external_url: Option<String>,
    pub perm: Option<String>,
    pub visible: Option<i16>,
    pub sort: Option<i32>,
    pub icon: Option<String>,
    pub redirect: Option<String>,
    pub keep_alive: Option<i16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuVisibleQuery {
    pub visible: i16,
}

/// AI 推断菜单配置入参
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MenuAiFillForm {
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub r#type: Option<String>,
    /// 上级菜单ID，用于参考同级菜单的命名风格
    pub parent_id: Option<i64>,
}

/// AI 推断的菜单配置
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuAiFillVO {
    pub route_path: Option<String>,
    pub perm: Option<String>,
    pub icon_keywords: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuVO {
    #[serde(serialize_with = "crate::util::i64_to_string")]
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub r#type: String,
    pub route_name: Option<String>,
    pub route_path: Option<String>,
    pub component: Option<String>,
    pub external_url: Option<String>,
    pub perm: Option<String>,
    pub visible: Option<i16>,
    pub sort: Option<i32>,
    pub icon: Option<String>,
    pub redirect: Option<String>,
    pub keep_alive: Option<i16>,
    pub children: Vec<MenuVO>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteVO {
    pub path: String,
    pub name: String,
    pub component: Option<String>,
    pub redirect: Option<String>,
    pub meta: RouteMetaVO,
    pub children: Vec<RouteVO>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMetaVO {
    pub title: String,
    pub icon: Option<String>,
    pub hidden: bool,
    #[serde(rename = "keepAlive")]
    pub keep_alive: bool,
    #[serde(rename = "externalUrl")]
    pub external_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuOptionVO {
    pub value: i64,
    pub label: String,
    pub parent_id: i64,
    pub children: Vec<MenuOptionVO>,
}
