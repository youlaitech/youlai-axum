//! 菜单管理服务：树形查询、路由生成

use sea_orm::*;
use sea_orm::sea_query::Expr;
use crate::common::enums::ResultCode;
use crate::entity;
use crate::framework::security::model::SecurityUserDetails;
use crate::framework::web::error::{AppError, AppResult};
use crate::modules::system::menu::dto::*;

pub struct MenuService<'a> { db: &'a DatabaseConnection }

impl<'a> MenuService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    /// 获取菜单树
    pub async fn get_tree(&self) -> AppResult<Vec<MenuVO>> {
        let all = entity::sys_menu::Entity::find().order_by_asc(entity::sys_menu::Column::Sort).all(self.db).await?;
        Ok(build_menu_tree(&all, 0))
    }

    /// 获取菜单表单
    pub async fn get_form(&self, id: i64) -> AppResult<MenuVO> {
        let m = entity::sys_menu::Entity::find_by_id(id).one(self.db).await?.unwrap();
        Ok(entity_to_vo(&m))
    }

    /// 新增菜单（自动计算 tree_path）
    pub async fn create(&self, f: &MenuCreate) -> AppResult<i64> {
        let tree_path = if f.parent_id == 0 { "0".to_string() } else {
            let parent = entity::sys_menu::Entity::find_by_id(f.parent_id).one(self.db).await?.unwrap();
            format!("{},{}", parent.tree_path.unwrap_or_default(), f.parent_id)
        };
        let a = entity::sys_menu::ActiveModel {
            parent_id: Set(f.parent_id), name: Set(f.name.clone()), r#type: Set(f.r#type.clone()),
            route_name: Set(f.route_name.clone()), route_path: Set(f.route_path.clone()),
            component: Set(if f.r#type == "C" { Some("Layout".to_string()) } else { f.component.clone() }), perm: Set(f.perm.clone()),
            visible: Set(f.visible.or(Some(1))), sort: Set(f.sort.or(Some(0))),
            icon: Set(f.icon.clone()), redirect: Set(f.redirect.clone()),
            keep_alive: Set(f.keep_alive.or(Some(0))), always_show: Set(f.always_show.or(Some(0))),
            tree_path: Set(Some(tree_path)), ..Default::default()
        };
        Ok(a.insert(self.db).await?.id)
    }

    /// 更新菜单
    pub async fn update(&self, f: &MenuUpdate) -> AppResult<()> {
        let mut a: entity::sys_menu::ActiveModel = entity::sys_menu::Entity::find_by_id(f.id).one(self.db).await?.unwrap().into();
        a.parent_id = Set(f.parent_id); a.name = Set(f.name.clone()); a.r#type = Set(f.r#type.clone());
        a.route_name = Set(f.route_name.clone()); a.route_path = Set(f.route_path.clone());
        a.component = Set(f.component.clone()); a.perm = Set(f.perm.clone());
        a.visible = Set(f.visible.or(Some(1))); a.sort = Set(f.sort.or(Some(0)));
        a.icon = Set(f.icon.clone()); a.redirect = Set(f.redirect.clone());
        a.update(self.db).await?; Ok(())
    }

    /// 更新菜单可见性
    pub async fn update_visible(&self, id: i64, visible: i16) -> AppResult<()> {
        entity::sys_menu::Entity::update_many().col_expr(entity::sys_menu::Column::Visible, Expr::value(visible)).filter(entity::sys_menu::Column::Id.eq(id)).exec(self.db).await?; Ok(())
    }

    /// 删除菜单（检查子节点），并清理角色关联
    pub async fn delete(&self, id: i64) -> AppResult<()> {
        let children = entity::sys_menu::Entity::find().filter(entity::sys_menu::Column::ParentId.eq(id)).count(self.db).await?;
        if children > 0 { return Err(AppError::Business(ResultCode::AccessPermissionException, Some("存在子菜单，无法删除".into()))); }
        entity::sys_menu::Entity::delete_by_id(id).exec(self.db).await?;
        entity::sys_role_menu::Entity::delete_many().filter(entity::sys_role_menu::Column::MenuId.eq(id)).exec(self.db).await?;
        Ok(())
    }

    /// 生成前端 Vue Router 动态路由（按当前用户角色过滤，已禁用角色不计入）
    pub async fn get_routes(&self, user: &SecurityUserDetails) -> AppResult<Vec<RouteVO>> {
        // 超级管理员 / ADMIN / ROOT 角色返回全部可见菜单
        if user.is_root || user.roles.iter().any(|r| r == "ADMIN" || r == "ROOT") {
            let all = entity::sys_menu::Entity::find()
                .filter(entity::sys_menu::Column::Visible.eq(1))
                .filter(entity::sys_menu::Column::Type.ne("B"))
                .order_by_asc(entity::sys_menu::Column::Sort)
                .all(self.db).await?;
            return Ok(build_route_tree(&all, 0));
        }

        // 查询用户已启用的角色
        let role_ids: Vec<i64> = entity::sys_user_role::Entity::find()
            .filter(entity::sys_user_role::Column::UserId.eq(user.user_id))
            .all(self.db).await?
            .into_iter().map(|r| r.role_id).collect();
        if role_ids.is_empty() {
            return Ok(vec![]);
        }
        let active_role_ids: Vec<i64> = entity::sys_role::Entity::find()
            .filter(entity::sys_role::Column::Id.is_in(role_ids))
            .filter(entity::sys_role::Column::Status.eq(1))
            .filter(entity::sys_role::Column::IsDeleted.eq(0))
            .all(self.db).await?
            .into_iter().map(|r| r.id).collect();
        if active_role_ids.is_empty() {
            return Ok(vec![]);
        }

        // 角色关联的菜单 ID
        let menu_ids: Vec<i64> = entity::sys_role_menu::Entity::find()
            .filter(entity::sys_role_menu::Column::RoleId.is_in(active_role_ids))
            .all(self.db).await?
            .into_iter().map(|rm| rm.menu_id).collect();
        if menu_ids.is_empty() {
            return Ok(vec![]);
        }

        let all = entity::sys_menu::Entity::find()
            .filter(entity::sys_menu::Column::Id.is_in(menu_ids))
            .filter(entity::sys_menu::Column::Visible.eq(1))
            .filter(entity::sys_menu::Column::Type.ne("B"))
            .order_by_asc(entity::sys_menu::Column::Sort)
            .all(self.db).await?;
        Ok(build_route_tree(&all, 0))
    }

    /// 菜单下拉选项（树形）
    pub async fn get_options(&self) -> AppResult<Vec<MenuOptionVO>> {
        let all = entity::sys_menu::Entity::find().order_by_asc(entity::sys_menu::Column::Sort).all(self.db).await?;
        Ok(build_option_tree(&all, 0))
    }
}

fn build_menu_tree(all: &[entity::sys_menu::Model], pid: i64) -> Vec<MenuVO> {
    all.iter().filter(|m| m.parent_id == pid).map(|m| MenuVO {
        id: m.id, parent_id: m.parent_id, name: m.name.clone(), r#type: m.r#type.clone(),
        route_name: m.route_name.clone(), route_path: m.route_path.clone(),
        component: m.component.clone(), perm: m.perm.clone(),
        visible: m.visible, sort: m.sort, icon: m.icon.clone(), redirect: m.redirect.clone(),
        keep_alive: m.keep_alive, always_show: m.always_show,
        children: build_menu_tree(all, m.id),
    }).collect()
}

fn entity_to_vo(m: &entity::sys_menu::Model) -> MenuVO {
    MenuVO { id: m.id, parent_id: m.parent_id, name: m.name.clone(), r#type: m.r#type.clone(),
        route_name: m.route_name.clone(), route_path: m.route_path.clone(),
        component: m.component.clone(), perm: m.perm.clone(),
        visible: m.visible, sort: m.sort, icon: m.icon.clone(), redirect: m.redirect.clone(),
        keep_alive: m.keep_alive, always_show: m.always_show, children: vec![] }
}

fn build_route_tree(all: &[entity::sys_menu::Model], pid: i64) -> Vec<RouteVO> {
    all.iter().filter(|m| m.parent_id == pid).map(|m| RouteVO {
        path: m.route_path.clone().unwrap_or_default(), name: m.route_name.clone().unwrap_or_default(),
        component: m.component.clone(), redirect: m.redirect.clone(),
        meta: RouteMetaVO { title: m.name.clone(), icon: m.icon.clone(), hidden: m.visible.unwrap_or(0) != 1, always_show: m.always_show.unwrap_or(0) == 1, keep_alive: m.keep_alive.unwrap_or(0) == 1 },
        children: build_route_tree(all, m.id),
    }).collect()
}

fn build_option_tree(all: &[entity::sys_menu::Model], pid: i64) -> Vec<MenuOptionVO> {
    all.iter().filter(|m| m.parent_id == pid).map(|m| MenuOptionVO { value: m.id, label: m.name.clone(), parent_id: m.parent_id, children: build_option_tree(all, m.id) }).collect()
}
