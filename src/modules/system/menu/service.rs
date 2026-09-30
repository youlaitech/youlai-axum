//! 菜单管理服务：树形查询、路由生成

use crate::ai;
use crate::config::AiConfig;
use crate::entity;
use crate::error::{AppError, AppResult};
use crate::modules::auth::principal::Principal;
use crate::modules::system::menu::dto::*;
use crate::result_code::ResultCode;
use sea_orm::sea_query::Expr;
use sea_orm::*;

/// AI 系统提示词名称
const MENU_SYSTEM_PROMPT_PATH: &str = "menu/system.md";
/// 同级菜单取样条数，样本只用于让模型沿用既有命名风格
const SIBLING_SAMPLE_LIMIT: u64 = 10;

/// 菜单类型取中文名称，无匹配时沿用原值
fn menu_type_label(menu_type: Option<&str>) -> String {
    match menu_type.unwrap_or("") {
        "C" => "目录".to_string(),
        "M" => "菜单".to_string(),
        "E" => "外链".to_string(),
        "B" => "按钮".to_string(),
        "" => "未指定".to_string(),
        other => other.to_string(),
    }
}

/// 值为空时取默认文案
fn blank_default(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|item| !item.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

pub struct MenuService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> MenuService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 获取菜单树
    pub async fn get_tree(&self) -> AppResult<Vec<MenuVO>> {
        let all = entity::sys_menu::Entity::find()
            .order_by_asc(entity::sys_menu::Column::Sort)
            .all(self.db)
            .await?;
        Ok(build_menu_tree(&all, 0))
    }

    /// 获取菜单表单
    pub async fn get_form(&self, id: i64) -> AppResult<MenuVO> {
        let m = entity::sys_menu::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .unwrap();
        Ok(entity_to_vo(&m))
    }

    /// 新增菜单
    pub async fn create(&self, f: &MenuCreate) -> AppResult<i64> {
        self.check_parent(f.parent_id, &f.r#type).await?;

        let is_external = f.r#type == "E";
        let is_embedded = is_external && f.component.as_deref() == Some("iframe");
        let needs_route_name = f.r#type == "M" || is_embedded;

        let route_name = if needs_route_name {
            if let Some(ref rn) = f.route_name {
                if !rn.is_empty() {
                    let exists = entity::sys_menu::Entity::find()
                        .filter(entity::sys_menu::Column::RouteName.eq(rn.as_str()))
                        .one(self.db)
                        .await?
                        .is_some();
                    if exists {
                        return Err(AppError::Business(ResultCode::AccessPermissionException, Some("路由名称已存在".into())));
                    }
                }
            }
            f.route_name.clone()
        } else {
            None
        };

        let route_path = if f.r#type == "C" && f.parent_id == 0 {
            f.route_path.as_ref().and_then(|p| {
                if p.is_empty() { None }
                else if !p.starts_with('/') { Some(format!("/{}", p)) }
                else { Some(p.clone()) }
            })
        } else {
            f.route_path.clone()
        };

        let component = if f.r#type == "C" {
            Some("Layout".to_string())
        } else if is_external && !is_embedded {
            None
        } else {
            f.component.clone()
        };

        let tree_path = if f.parent_id == 0 {
            "0".to_string()
        } else {
            let parent = entity::sys_menu::Entity::find_by_id(f.parent_id)
                .one(self.db)
                .await?
                .unwrap();
            format!("{},{}", parent.tree_path.unwrap_or_default(), f.parent_id)
        };

        let a = entity::sys_menu::ActiveModel {
            parent_id: Set(f.parent_id),
            name: Set(f.name.clone()),
            r#type: Set(f.r#type.clone()),
            route_name: Set(route_name),
            route_path: Set(route_path),
            component: Set(component),
            external_url: Set(f.external_url.clone()),
            perm: Set(f.perm.clone()),
            visible: Set(f.visible.or(Some(1))),
            sort: Set(Some(match f.sort {
                Some(s) => s,
                None => self.next_sort(f.parent_id).await?,
            })),
            icon: Set(f.icon.clone()),
            redirect: Set(f.redirect.clone()),
            keep_alive: Set(f.keep_alive.or(Some(0))),
            tree_path: Set(Some(tree_path.clone())),
            ..Default::default()
        };
        let id = a.insert(self.db).await?.id;

        // 新增页面菜单时按需生成增删改查按钮
        if f.generate_crud_buttons.unwrap_or(false) {
            if let Some(prefix) = f
                .button_perm_prefix
                .as_deref()
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                let button_tree_path = format!("{},{}", tree_path, id);
                self.save_crud_buttons(id, prefix, &button_tree_path).await?;
            }
        }

        Ok(id)
    }

    /// 更新菜单
    pub async fn update(&self, f: &MenuUpdate) -> AppResult<()> {
        if f.parent_id == f.id {
            return Err(AppError::Business(ResultCode::AccessPermissionException, Some("父级菜单不能为当前菜单".into())));
        }

        // 父级变化时校验层级（父级未变时跳过，兼容历史遗留的非法层级）
        if let Some(current) = entity::sys_menu::Entity::find_by_id(f.id).one(self.db).await? {
            if current.parent_id != f.parent_id {
                self.check_parent(f.parent_id, &f.r#type).await?;
            }
        }

        let mut a: entity::sys_menu::ActiveModel = entity::sys_menu::Entity::find_by_id(f.id)
            .one(self.db)
            .await?
            .unwrap()
            .into();

        let is_external = f.r#type == "E";
        let is_embedded = is_external && f.component.as_deref() == Some("iframe");
        let needs_route_name = f.r#type == "M" || is_embedded;

        let route_name = if needs_route_name {
            if let Some(ref rn) = f.route_name {
                if !rn.is_empty() {
                    let exists = entity::sys_menu::Entity::find()
                        .filter(entity::sys_menu::Column::RouteName.eq(rn.as_str()))
                        .filter(entity::sys_menu::Column::Id.ne(f.id))
                        .one(self.db)
                        .await?
                        .is_some();
                    if exists {
                        return Err(AppError::Business(ResultCode::AccessPermissionException, Some("路由名称已存在".into())));
                    }
                }
            }
            f.route_name.clone()
        } else {
            None
        };

        a.parent_id = Set(f.parent_id);
        a.name = Set(f.name.clone());
        a.r#type = Set(f.r#type.clone());
        a.route_name = Set(route_name);
        a.route_path = Set(f.route_path.clone());
        a.component = Set(if is_external && !is_embedded {
            None
        } else {
            f.component.clone()
        });
        a.external_url = Set(f.external_url.clone());
        a.perm = Set(f.perm.clone());
        a.visible = Set(f.visible.or(Some(1)));
        a.sort = Set(f.sort.or(Some(0)));
        a.icon = Set(f.icon.clone());
        a.redirect = Set(f.redirect.clone());
        a.keep_alive = Set(f.keep_alive.or(Some(0)));
        a.update(self.db).await?;
        Ok(())
    }

    /// 更新菜单可见性
    pub async fn update_visible(&self, id: i64, visible: i16) -> AppResult<()> {
        entity::sys_menu::Entity::update_many()
            .col_expr(entity::sys_menu::Column::Visible, Expr::value(visible))
            .filter(entity::sys_menu::Column::Id.eq(id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 删除菜单（级联删除子菜单）并清理角色关联
    pub async fn delete(&self, id: i64) -> AppResult<()> {
        // 级联范围：自身 + tree_path 祖先链里带上该菜单ID的后代
        let ids: Vec<i64> = entity::sys_menu::Entity::find()
            .select_only()
            .column(entity::sys_menu::Column::Id)
            .filter(
                Condition::any()
                    .add(entity::sys_menu::Column::Id.eq(id))
                    .add(entity::sys_menu::Column::TreePath.like(format!("{id},%")))
                    .add(entity::sys_menu::Column::TreePath.like(format!("%,{id},%")))
                    .add(entity::sys_menu::Column::TreePath.like(format!("%,{id}"))),
            )
            .into_tuple()
            .all(self.db)
            .await?;

        if ids.is_empty() {
            return Err(AppError::Business(ResultCode::DataNotFound, None));
        }

        entity::sys_role_menu::Entity::delete_many()
            .filter(entity::sys_role_menu::Column::MenuId.is_in(ids.clone()))
            .exec(self.db)
            .await?;
        entity::sys_menu::Entity::delete_many()
            .filter(entity::sys_menu::Column::Id.is_in(ids))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// 生成前端 Vue Router 动态路由（按当前用户角色过滤，已禁用角色不计入）
    pub async fn get_routes(&self, user: &Principal) -> AppResult<Vec<RouteVO>> {
        // 超级管理员 / ADMIN / ROOT 角色返回全部可见菜单
        if user.is_root || user.roles.iter().any(|r| r == "ADMIN" || r == "ROOT") {
            let all = entity::sys_menu::Entity::find()
                .filter(entity::sys_menu::Column::Visible.eq(1))
                .filter(entity::sys_menu::Column::Type.ne("B"))
                .order_by_asc(entity::sys_menu::Column::Sort)
                .all(self.db)
                .await?;
            return Ok(build_route_tree(&all, 0));
        }

        // 查询用户已启用的角色
        let role_ids: Vec<i64> = entity::sys_user_role::Entity::find()
            .filter(entity::sys_user_role::Column::UserId.eq(user.user_id))
            .all(self.db)
            .await?
            .into_iter()
            .map(|r| r.role_id)
            .collect();
        if role_ids.is_empty() {
            return Ok(vec![]);
        }
        let active_role_ids: Vec<i64> = entity::sys_role::Entity::find()
            .filter(entity::sys_role::Column::Id.is_in(role_ids))
            .filter(entity::sys_role::Column::Status.eq(1))
            .filter(entity::sys_role::Column::IsDeleted.eq(0))
            .all(self.db)
            .await?
            .into_iter()
            .map(|r| r.id)
            .collect();
        if active_role_ids.is_empty() {
            return Ok(vec![]);
        }

        // 角色关联的菜单 ID
        let menu_ids: Vec<i64> = entity::sys_role_menu::Entity::find()
            .filter(entity::sys_role_menu::Column::RoleId.is_in(active_role_ids))
            .all(self.db)
            .await?
            .into_iter()
            .map(|rm| rm.menu_id)
            .collect();
        if menu_ids.is_empty() {
            return Ok(vec![]);
        }

        let all = entity::sys_menu::Entity::find()
            .filter(entity::sys_menu::Column::Id.is_in(menu_ids))
            .filter(entity::sys_menu::Column::Visible.eq(1))
            .filter(entity::sys_menu::Column::Type.ne("B"))
            .order_by_asc(entity::sys_menu::Column::Sort)
            .all(self.db)
            .await?;
        Ok(build_route_tree(&all, 0))
    }

    /// 菜单下拉选项（树形，types 为空查全部）
    pub async fn get_options(&self, types: Option<Vec<String>>) -> AppResult<Vec<MenuOptionVO>> {
        let mut select = entity::sys_menu::Entity::find();
        if let Some(types) = types.filter(|t| !t.is_empty()) {
            select = select.filter(entity::sys_menu::Column::Type.is_in(types));
        }
        let all = select
            .order_by_asc(entity::sys_menu::Column::Sort)
            .all(self.db)
            .await?;
        Ok(build_option_tree(&all, 0))
    }

    /// 校验上级菜单层级：按钮只能挂在菜单下，其他类型只能挂在顶级或目录下
    async fn check_parent(&self, parent_id: i64, menu_type: &str) -> AppResult<()> {
        let is_button = menu_type == "B";
        if parent_id == 0 {
            if is_button {
                return Err(AppError::Business(
                    ResultCode::AccessPermissionException,
                    Some("按钮权限只能挂在菜单下".into()),
                ));
            }
            return Ok(());
        }

        let parent = entity::sys_menu::Entity::find_by_id(parent_id)
            .one(self.db)
            .await?
            .ok_or_else(|| {
                AppError::Business(
                    ResultCode::AccessPermissionException,
                    Some("上级菜单不存在".into()),
                )
            })?;
        let parent_is_menu = parent.r#type == "M";
        if is_button && !parent_is_menu {
            return Err(AppError::Business(
                ResultCode::AccessPermissionException,
                Some("按钮权限只能挂在菜单下".into()),
            ));
        }
        if !is_button && parent_is_menu {
            return Err(AppError::Business(
                ResultCode::AccessPermissionException,
                Some("菜单下只能挂按钮权限".into()),
            ));
        }
        Ok(())
    }

    /// 同级菜单的最大排序 + 1
    async fn next_sort(&self, parent_id: i64) -> AppResult<i32> {
        let max_sort = entity::sys_menu::Entity::find()
            .filter(entity::sys_menu::Column::ParentId.eq(parent_id))
            .order_by_desc(entity::sys_menu::Column::Sort)
            .one(self.db)
            .await?
            .and_then(|m| m.sort);
        Ok(max_sort.map_or(1, |s| s + 1))
    }

    /// 生成增删改查按钮权限
    async fn save_crud_buttons(
        &self,
        menu_id: i64,
        perm_prefix: &str,
        button_tree_path: &str,
    ) -> AppResult<()> {
        let names = ["查询", "新增", "修改", "删除"];
        let actions = ["list", "create", "update", "delete"];
        for (i, (name, action)) in names.iter().zip(actions.iter()).enumerate() {
            entity::sys_menu::ActiveModel {
                parent_id: Set(menu_id),
                r#type: Set("B".to_string()),
                name: Set((*name).to_string()),
                perm: Set(Some(format!("{perm_prefix}:{action}"))),
                sort: Set(Some(i as i32 + 1)),
                visible: Set(Some(1)),
                tree_path: Set(Some(button_tree_path.to_string())),
                ..Default::default()
            }
            .insert(self.db)
            .await?;
        }
        Ok(())
    }

    /// 用 AI 推断菜单的访问路径、权限标识与图标，命名风格参照同级菜单
    pub async fn ai_fill(
        &self,
        http: &reqwest::Client,
        ai_config: &AiConfig,
        f: &MenuAiFillForm,
    ) -> AppResult<MenuAiFillVO> {
        let system_prompt = ai::prompt(MENU_SYSTEM_PROMPT_PATH)?;
        let user_prompt = self.build_ai_user_prompt(f).await?;
        let value = ai::chat_json_list(http, ai_config, system_prompt, &user_prompt).await?;

        let icon_keywords = value["iconKeywords"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        Ok(MenuAiFillVO {
            route_path: value["routePath"].as_str().map(str::to_string),
            perm: value["perm"].as_str().map(str::to_string),
            icon_keywords,
        })
    }

    /// 构造用户提示词，附带同级菜单样本供模型沿用既有命名风格
    async fn build_ai_user_prompt(&self, f: &MenuAiFillForm) -> AppResult<String> {
        let parent_id = f.parent_id.unwrap_or(0);

        let parent_name = if parent_id > 0 {
            entity::sys_menu::Entity::find_by_id(parent_id)
                .one(self.db)
                .await?
                .map(|menu| menu.name)
                .unwrap_or_else(|| "顶级".to_string())
        } else {
            "顶级".to_string()
        };

        let siblings = entity::sys_menu::Entity::find()
            .filter(entity::sys_menu::Column::ParentId.eq(parent_id))
            .order_by_asc(entity::sys_menu::Column::Sort)
            .limit(SIBLING_SAMPLE_LIMIT)
            .all(self.db)
            .await?;

        let mut text = String::new();
        text.push_str(&format!("菜单名称：{}\n", f.name));
        text.push_str(&format!("菜单类型：{}\n", menu_type_label(f.r#type.as_deref())));
        text.push_str(&format!("上级菜单：{parent_name}\n"));
        text.push_str("同级菜单示例：\n");
        if siblings.is_empty() {
            text.push_str("（无）\n");
        } else {
            for menu in siblings {
                // 示例路径只展示当前这一级的路径片段
                let route_path = menu.route_path.as_deref().unwrap_or("").trim_start_matches('/');
                text.push_str(&format!(
                    "- {}（类型 {}，路径片段 {}，权限 {}）\n",
                    menu.name,
                    menu_type_label(Some(menu.r#type.as_str())),
                    blank_default(Some(route_path), "无"),
                    blank_default(menu.perm.as_deref(), "无"),
                ));
            }
        }
        Ok(text)
    }
}

fn build_menu_tree(all: &[entity::sys_menu::Model], pid: i64) -> Vec<MenuVO> {
    all.iter()
        .filter(|m| m.parent_id == pid)
        .map(|m| MenuVO {
            id: m.id,
            parent_id: m.parent_id,
            name: m.name.clone(),
            r#type: m.r#type.clone(),
            route_name: m.route_name.clone(),
            route_path: m.route_path.clone(),
            component: m.component.clone(),
            external_url: m.external_url.clone(),
            perm: m.perm.clone(),
            visible: m.visible,
            sort: m.sort,
            icon: m.icon.clone(),
            redirect: m.redirect.clone(),
            keep_alive: m.keep_alive,
            children: build_menu_tree(all, m.id),
        })
        .collect()
}

fn entity_to_vo(m: &entity::sys_menu::Model) -> MenuVO {
    MenuVO {
        id: m.id,
        parent_id: m.parent_id,
        name: m.name.clone(),
        r#type: m.r#type.clone(),
        route_name: m.route_name.clone(),
        route_path: m.route_path.clone(),
        component: m.component.clone(),
        external_url: m.external_url.clone(),
        perm: m.perm.clone(),
        visible: m.visible,
        sort: m.sort,
        icon: m.icon.clone(),
        redirect: m.redirect.clone(),
        keep_alive: m.keep_alive,
        children: vec![],
    }
}

fn build_route_tree(all: &[entity::sys_menu::Model], pid: i64) -> Vec<RouteVO> {
    all.iter()
        .filter(|m| m.parent_id == pid)
        .map(|m| {
            // 外链(E)：非内嵌 iframe 时路径直接用 externalUrl（以 http 开头，前端新标签打开）
            let is_external = m.r#type == "E";
            let is_embedded = is_external && m.component.as_deref() == Some("iframe");
            let path = if is_external && !is_embedded {
                m.external_url
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| m.route_path.clone().unwrap_or_default())
            } else {
                m.route_path.clone().unwrap_or_default()
            };
            RouteVO {
            path,
            name: m.route_name.clone().unwrap_or_default(),
            component: m.component.clone(),
            redirect: m.redirect.clone(),
            meta: RouteMetaVO {
                title: m.name.clone(),
                icon: m.icon.clone(),
                hidden: m.visible.unwrap_or(0) != 1,
                keep_alive: m.keep_alive.unwrap_or(0) == 1,
                external_url: if is_external { m.external_url.clone() } else { None },
            },
            children: build_route_tree(all, m.id),
            }
        })
        .collect()
}

fn build_option_tree(all: &[entity::sys_menu::Model], pid: i64) -> Vec<MenuOptionVO> {
    all.iter()
        .filter(|m| m.parent_id == pid)
        .map(|m| MenuOptionVO {
            value: m.id,
            label: m.name.clone(),
            parent_id: m.parent_id,
            children: build_option_tree(all, m.id),
        })
        .collect()
}
