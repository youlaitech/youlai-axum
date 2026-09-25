//! 动态表单服务：定义 CRUD、发布停用、渲染、访问菜单生成与表单数据

use crate::ai;
use crate::config::AiConfig;
use crate::entity;
use crate::error::{AppError, AppResult};
use crate::modules::form::dto::*;
use crate::pagination::PageData;
use crate::result_code::ResultCode;
use chrono::Utc;
use redis::aio::ConnectionManager;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use serde_json::{json, Map, Value};

const STATUS_DRAFT: i16 = 0;
const STATUS_PUBLISHED: i16 = 1;
const STATUS_DISABLED: i16 = -1;
/// 默认挂载目录名称
const DEFAULT_CATALOG_NAME: &str = "表单中心";
/// 表单管理目录名称（顶级，dynamic-form.sql 预置）
const FORM_ADMIN_CATALOG_NAME: &str = "动态表单";
/// 系统提示词名称
const SYSTEM_PROMPT_PATH: &str = "form/system.md";
/// 公开表单单 IP 提交上限（次/分钟）
const PUBLIC_SUBMIT_LIMIT: i64 = 10;

pub struct FormService<'a> {
    db: &'a DatabaseConnection,
    redis: &'a ConnectionManager,
    http: &'a reqwest::Client,
    ai: &'a AiConfig,
}

impl<'a> FormService<'a> {
    pub fn new(
        db: &'a DatabaseConnection,
        redis: &'a ConnectionManager,
        http: &'a reqwest::Client,
        ai: &'a AiConfig,
    ) -> Self {
        Self {
            db,
            redis,
            http,
            ai,
        }
    }

    // ---------------------------------- 表单定义 ----------------------------------

    /// 表单定义分页列表
    pub async fn get_page(&self, q: &FormDefinitionQuery) -> AppResult<PageData<FormDefinitionVO>> {
        let mut select = entity::form_definition::Entity::find()
            .filter(entity::form_definition::Column::IsDeleted.eq(0));
        if let Some(ref kw) = q.keywords {
            if !kw.is_empty() {
                select = select.filter(
                    Condition::any()
                        .add(entity::form_definition::Column::FormName.like(format!("%{kw}%")))
                        .add(entity::form_definition::Column::FormKey.like(format!("%{kw}%"))),
                );
            }
        }
        if let Some(status) = q.status {
            select = select.filter(entity::form_definition::Column::Status.eq(status));
        }
        if let Some(ref category) = q.category {
            if !category.is_empty() {
                select = select.filter(entity::form_definition::Column::Category.eq(category.as_str()));
            }
        }

        let total = select.clone().count(self.db).await?;
        let rows = select
            .order_by_desc(entity::form_definition::Column::CreateTime)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;

        Ok(PageData::new(
            rows.iter().map(to_definition_vo).collect(),
            total,
        ))
    }

    /// 表单定义表单数据
    pub async fn get_form(&self, id: i64) -> AppResult<FormDefinitionForm> {
        let m = self.find_definition(id).await?;
        Ok(FormDefinitionForm {
            id: Some(m.id),
            form_key: m.form_key.clone(),
            form_name: m.form_name.clone(),
            description: m.description.clone(),
            form_json: Some(m.form_json.to_string()),
            options_json: m.options_json.as_ref().map(|v| v.to_string()),
            is_public: Some(m.is_public),
            category: Some(m.category.clone()),
        })
    }

    /// 新增表单定义（初始为草稿态）
    pub async fn create(&self, f: &FormDefinitionForm) -> AppResult<i64> {
        let form_key = f.form_key.trim();
        let exists = entity::form_definition::Entity::find()
            .filter(entity::form_definition::Column::FormKey.eq(form_key))
            .filter(entity::form_definition::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .is_some();
        if exists {
            return Err(bad("表单标识已存在"));
        }

        let now = Utc::now().naive_utc();
        let id = entity::form_definition::ActiveModel {
            form_key: Set(form_key.to_string()),
            form_name: Set(f.form_name.trim().to_string()),
            description: Set(f.description.clone()),
            form_json: Set(parse_json_or_empty_array(f.form_json.as_deref())),
            options_json: Set(f.options_json.as_deref().and_then(|s| serde_json::from_str(s).ok())),
            status: Set(STATUS_DRAFT),
            is_public: Set(f.is_public.unwrap_or(0)),
            category: Set(f
                .category
                .clone()
                .filter(|c| !c.is_empty())
                .unwrap_or_else(|| "normal".to_string())),
            version: Set(1),
            create_time: Set(Some(now)),
            update_time: Set(Some(now)),
            ..Default::default()
        }
        .insert(self.db)
        .await?
        .id;

        Ok(id)
    }

    /// 修改表单定义（formKey 不可修改；草稿态可改元数据与规则）
    pub async fn update(&self, f: &FormDefinitionForm) -> AppResult<()> {
        let id = f.id.ok_or_else(|| bad("表单ID不能为空"))?;
        let current = self.find_definition(id).await?;

        if !f.form_key.is_empty() && f.form_key.trim() != current.form_key {
            return Err(bad("表单标识创建后不可修改"));
        }

        let mut active: entity::form_definition::ActiveModel = current.clone().into();
        if current.status == STATUS_DRAFT {
            active.form_name = Set(f.form_name.trim().to_string());
            active.description = Set(f.description.clone());
            if let Some(ref category) = f.category {
                if !category.is_empty() {
                    active.category = Set(category.clone());
                }
            }
        }
        if let Some(ref form_json) = f.form_json {
            active.form_json = Set(parse_json_or_empty_array(Some(form_json)));
        }
        if f.options_json.is_some() {
            active.options_json = Set(f.options_json.as_deref().and_then(|s| serde_json::from_str(s).ok()));
        }
        if let Some(is_public) = f.is_public {
            active.is_public = Set(is_public);
        }
        active.update_time = Set(Some(Utc::now().naive_utc()));
        active.update(self.db).await?;

        Ok(())
    }

    /// 批量删除表单定义（级联逻辑删除访问菜单、提交数据与版本快照）
    pub async fn delete(&self, ids: &str) -> AppResult<()> {
        let id_list = parse_ids(ids);
        if id_list.is_empty() {
            return Err(bad("删除的表单数据为空"));
        }

        let forms = entity::form_definition::Entity::find()
            .filter(entity::form_definition::Column::Id.is_in(id_list.clone()))
            .filter(entity::form_definition::Column::IsDeleted.eq(0))
            .all(self.db)
            .await?;

        for form in &forms {
            if let Some(menu_id) = form.menu_id {
                entity::sys_menu::Entity::delete_many()
                    .filter(entity::sys_menu::Column::Id.eq(menu_id))
                    .exec(self.db)
                    .await?;
                entity::sys_role_menu::Entity::delete_many()
                    .filter(entity::sys_role_menu::Column::MenuId.eq(menu_id))
                    .exec(self.db)
                    .await?;
            }
        }

        entity::form_definition::Entity::update_many()
            .col_expr(entity::form_definition::Column::IsDeleted, Expr::value(1))
            .filter(entity::form_definition::Column::Id.is_in(id_list.clone()))
            .exec(self.db)
            .await?;
        entity::form_data::Entity::update_many()
            .col_expr(entity::form_data::Column::IsDeleted, Expr::value(1))
            .filter(entity::form_data::Column::FormId.is_in(id_list.clone()))
            .exec(self.db)
            .await?;
        entity::form_snapshot::Entity::update_many()
            .col_expr(entity::form_snapshot::Column::IsDeleted, Expr::value(1))
            .filter(entity::form_snapshot::Column::FormId.is_in(id_list))
            .exec(self.db)
            .await?;

        Ok(())
    }

    /// 发布表单（版本号+1，固化版本快照）
    pub async fn publish(&self, id: i64) -> AppResult<()> {
        self.find_definition(id).await?;

        entity::form_definition::Entity::update_many()
            .col_expr(
                entity::form_definition::Column::Status,
                Expr::value(STATUS_PUBLISHED),
            )
            .col_expr(
                entity::form_definition::Column::Version,
                Expr::col(entity::form_definition::Column::Version).add(1),
            )
            .col_expr(
                entity::form_definition::Column::UpdateTime,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(entity::form_definition::Column::Id.eq(id))
            .exec(self.db)
            .await?;

        let published = self.find_definition(id).await?;
        // 固化版本快照：数据回显按提交时版本加载，防止规则变更导致历史数据漂移
        entity::form_snapshot::ActiveModel {
            form_id: Set(id),
            version: Set(published.version),
            form_json: Set(published.form_json.clone()),
            options_json: Set(published.options_json.clone()),
            create_time: Set(Some(Utc::now().naive_utc())),
            update_time: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        }
        .insert(self.db)
        .await?;

        Ok(())
    }

    /// 停用表单
    pub async fn disable(&self, id: i64) -> AppResult<()> {
        self.find_definition(id).await?;

        entity::form_definition::Entity::update_many()
            .col_expr(
                entity::form_definition::Column::Status,
                Expr::value(STATUS_DISABLED),
            )
            .col_expr(
                entity::form_definition::Column::UpdateTime,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(entity::form_definition::Column::Id.eq(id))
            .exec(self.db)
            .await?;

        Ok(())
    }

    /// 按 formKey 获取已发布表单的渲染规则
    pub async fn render(&self, form_key: &str) -> AppResult<FormRenderVO> {
        let m = entity::form_definition::Entity::find()
            .filter(entity::form_definition::Column::FormKey.eq(form_key))
            .filter(entity::form_definition::Column::Status.eq(STATUS_PUBLISHED))
            .filter(entity::form_definition::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| bad("表单不存在或未发布"))?;

        Ok(to_render_vo(&m))
    }

    /// 按 formKey 获取公开表单的渲染规则（仅 is_public=1 且已发布）
    pub async fn public_render(&self, form_key: &str) -> AppResult<FormRenderVO> {
        let m = entity::form_definition::Entity::find()
            .filter(entity::form_definition::Column::FormKey.eq(form_key))
            .filter(entity::form_definition::Column::Status.eq(STATUS_PUBLISHED))
            .filter(entity::form_definition::Column::IsPublic.eq(1))
            .filter(entity::form_definition::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| bad("表单不存在或未开放公开访问"))?;

        Ok(to_render_vo(&m))
    }

    /// 表单已生成的菜单配置（发布向导回显）
    pub async fn get_menu(&self, form_id: i64) -> AppResult<Option<FormMenuVO>> {
        let form = self.find_definition(form_id).await?;
        let Some(menu_id) = form.menu_id else {
            return Ok(None);
        };
        let Some(menu) = entity::sys_menu::Entity::find_by_id(menu_id).one(self.db).await? else {
            return Ok(None);
        };

        let role_ids = entity::sys_role_menu::Entity::find()
            .filter(entity::sys_role_menu::Column::MenuId.eq(menu.id))
            .all(self.db)
            .await?
            .iter()
            .map(|r| r.role_id.to_string())
            .collect();

        Ok(Some(FormMenuVO {
            menu_id: menu.id,
            menu_name: menu.name.clone(),
            parent_id: menu.parent_id,
            role_ids,
        }))
    }

    /// 生成表单访问菜单并授权可见角色（幂等：已生成时更新原菜单）
    pub async fn save_menu(&self, form_id: i64, f: &FormMenuForm) -> AppResult<()> {
        let form = self.find_definition(form_id).await?;

        let parent = match f.parent_id {
            Some(parent_id) if parent_id != 0 => {
                let parent = entity::sys_menu::Entity::find_by_id(parent_id)
                    .one(self.db)
                    .await?
                    .ok_or_else(|| bad("上级菜单不存在"))?;
                if parent.r#type != "C" {
                    return Err(bad("上级菜单必须为目录类型"));
                }
                parent
            }
            _ => self.ensure_default_catalog().await?,
        };

        let route_name = format!("FormRender{}", to_upper_camel(&form.form_key));
        let now = Utc::now().naive_utc();
        let params = json!({ "formKey": form.form_key });

        // 已生成过时走更新语义（幂等）
        let menu_id = match form.menu_id {
            Some(menu_id) => {
                entity::sys_menu::Entity::update_many()
                    .col_expr(
                        entity::sys_menu::Column::ParentId,
                        Expr::value(parent.id),
                    )
                    .col_expr(
                        entity::sys_menu::Column::Name,
                        Expr::value(f.menu_name.trim().to_string()),
                    )
                    .col_expr(entity::sys_menu::Column::RouteName, Expr::value(route_name.clone()))
                    .col_expr(
                        entity::sys_menu::Column::RoutePath,
                        Expr::value(form.form_key.clone()),
                    )
                    .col_expr(
                        entity::sys_menu::Column::Component,
                        Expr::value("dynamic-form/render".to_string()),
                    )
                    .col_expr(entity::sys_menu::Column::Params, Expr::value(params.clone()))
                    .col_expr(
                        entity::sys_menu::Column::TreePath,
                        Expr::value(catalog_tree_path(&parent)),
                    )
                    .col_expr(entity::sys_menu::Column::UpdateTime, Expr::value(now))
                    .filter(entity::sys_menu::Column::Id.eq(menu_id))
                    .exec(self.db)
                    .await?;
                menu_id
            }
            None => {
                let tree_path = catalog_tree_path(&parent);
                entity::sys_menu::ActiveModel {
                    parent_id: Set(parent.id),
                    name: Set(f.menu_name.trim().to_string()),
                    r#type: Set("M".to_string()),
                    route_name: Set(Some(route_name)),
                    route_path: Set(Some(form.form_key.clone())),
                    component: Set(Some("dynamic-form/render".to_string())),
                    params: Set(Some(params)),
                    visible: Set(Some(1)),
                    sort: Set(Some(1)),
                    keep_alive: Set(Some(0)),
                    tree_path: Set(Some(tree_path)),
                    create_time: Set(Some(now)),
                    update_time: Set(Some(now)),
                    ..Default::default()
                }
                .insert(self.db)
                .await?
                .id
            }
        };

        // 回写 menu_id：下次发布走更新语义，防重复生成菜单
        entity::form_definition::Entity::update_many()
            .col_expr(entity::form_definition::Column::MenuId, Expr::value(menu_id))
            .col_expr(entity::form_definition::Column::UpdateTime, Expr::value(now))
            .filter(entity::form_definition::Column::Id.eq(form_id))
            .exec(self.db)
            .await?;

        if let Some(ref role_ids) = f.role_ids {
            self.assign_menu_to_roles(menu_id, role_ids).await?;
        }

        Ok(())
    }

    /// 审批表单下拉选项（仅已发布且类型为 workflow 的表单）
    pub async fn workflow_options(&self) -> AppResult<Vec<Value>> {
        let rows = entity::form_definition::Entity::find()
            .filter(entity::form_definition::Column::Category.eq("workflow"))
            .filter(entity::form_definition::Column::Status.eq(STATUS_PUBLISHED))
            .filter(entity::form_definition::Column::IsDeleted.eq(0))
            .order_by_asc(entity::form_definition::Column::CreateTime)
            .all(self.db)
            .await?;

        Ok(rows
            .iter()
            .map(|m| json!({ "value": m.form_key, "label": m.form_name }))
            .collect())
    }

    /// 用 AI 把需求描述转成 form-create 规则（产出经结构校验后才返回）
    pub async fn ai_generate_rule(&self, description: &str) -> AppResult<String> {
        let system_prompt = ai::prompt(SYSTEM_PROMPT_PATH)?;
        let user_prompt = format!("需求描述：\n{description}");
        let rules = ai::chat_json_list(self.http, self.ai, system_prompt, &user_prompt).await?;
        validate_rule(&rules.to_string())
    }

    // ---------------------------------- 表单数据 ----------------------------------

    /// 提交表单数据（仅已发布表单可提交，提交时快照表单版本）
    pub async fn save_data(
        &self,
        form_key: &str,
        data: &Value,
        user_id: Option<i64>,
    ) -> AppResult<()> {
        let form = self.find_by_key(form_key).await?;
        if form.status != STATUS_PUBLISHED {
            return Err(bad("表单未发布或已停用"));
        }
        self.persist_data(&form, data, user_id).await
    }

    /// 匿名提交公开表单数据（formKey + IP 维度限流）
    pub async fn save_public_data(&self, form_key: &str, data: &Value, ip: &str) -> AppResult<()> {
        let form = self.find_by_key(form_key).await?;
        if form.status != STATUS_PUBLISHED || form.is_public != 1 {
            return Err(bad("表单不存在或未开放公开访问"));
        }

        if !self.check_public_submit_limit(form_key, ip).await {
            return Err(bad("提交过于频繁，请稍后再试"));
        }

        tracing::info!("公开表单匿名提交 formKey={form_key} ip={ip}");
        self.persist_data(&form, data, None).await
    }

    /// 表单数据分页列表（联查提交人昵称）
    pub async fn data_page(&self, form_key: &str, q: &FormDataQuery) -> AppResult<PageData<FormDataVO>> {
        let form = self.find_by_key(form_key).await?;

        let select = entity::form_data::Entity::find()
            .filter(entity::form_data::Column::FormId.eq(form.id))
            .filter(entity::form_data::Column::IsDeleted.eq(0));

        let total = select.clone().count(self.db).await?;
        let rows = select
            .order_by_desc(entity::form_data::Column::CreateTime)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            items.push(self.to_data_vo(row, false).await?);
        }

        Ok(PageData::new(items, total))
    }

    /// 表单数据详情（只读回显，规则取提交时版本的快照）
    pub async fn data_detail(&self, id: i64) -> AppResult<FormDataVO> {
        let row = entity::form_data::Entity::find_by_id(id)
            .filter(entity::form_data::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| bad("表单数据不存在"))?;

        self.to_data_vo(&row, true).await
    }

    /// 批量删除表单数据（逻辑删除）
    pub async fn delete_data(&self, ids: &str) -> AppResult<()> {
        let id_list = parse_ids(ids);
        if id_list.is_empty() {
            return Err(bad("删除的表单数据为空"));
        }

        entity::form_data::Entity::update_many()
            .col_expr(entity::form_data::Column::IsDeleted, Expr::value(1))
            .filter(entity::form_data::Column::Id.is_in(id_list))
            .filter(entity::form_data::Column::IsDeleted.eq(0))
            .exec(self.db)
            .await?;

        Ok(())
    }

    // ---------------------------------- 内部方法 ----------------------------------

    async fn find_definition(&self, id: i64) -> AppResult<entity::form_definition::Model> {
        entity::form_definition::Entity::find_by_id(id)
            .filter(entity::form_definition::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| bad("表单不存在"))
    }

    async fn find_by_key(&self, form_key: &str) -> AppResult<entity::form_definition::Model> {
        entity::form_definition::Entity::find()
            .filter(entity::form_definition::Column::FormKey.eq(form_key))
            .filter(entity::form_definition::Column::IsDeleted.eq(0))
            .one(self.db)
            .await?
            .ok_or_else(|| bad("表单不存在"))
    }

    /// 数据落库（登录/匿名提交共用）
    async fn persist_data(
        &self,
        form: &entity::form_definition::Model,
        data: &Value,
        user_id: Option<i64>,
    ) -> AppResult<()> {
        let filtered = validate_and_filter(&form.form_json, data)?;
        let now = Utc::now().naive_utc();

        entity::form_data::ActiveModel {
            form_id: Set(form.id),
            form_version: Set(form.version),
            data_json: Set(filtered),
            create_by: Set(user_id),
            create_time: Set(Some(now)),
            update_time: Set(Some(now)),
            ..Default::default()
        }
        .insert(self.db)
        .await?;

        Ok(())
    }

    async fn to_data_vo(
        &self,
        row: &entity::form_data::Model,
        with_rule: bool,
    ) -> AppResult<FormDataVO> {
        let create_by_name = match row.create_by {
            Some(create_by) => entity::sys_user::Entity::find_by_id(create_by)
                .one(self.db)
                .await?
                .and_then(|u| u.nickname),
            None => None,
        };

        let (form_json, options_json) = if with_rule {
            // 命中版本快照用快照规则，否则回退当前定义（兼容快照表启用前提交的历史数据）
            let snapshot = entity::form_snapshot::Entity::find()
                .filter(entity::form_snapshot::Column::FormId.eq(row.form_id))
                .filter(entity::form_snapshot::Column::Version.eq(row.form_version))
                .filter(entity::form_snapshot::Column::IsDeleted.eq(0))
                .one(self.db)
                .await?;
            match snapshot {
                Some(s) => (Some(s.form_json), s.options_json),
                None => match entity::form_definition::Entity::find_by_id(row.form_id)
                    .one(self.db)
                    .await?
                {
                    Some(f) => (Some(f.form_json), f.options_json),
                    None => (None, None),
                },
            }
        } else {
            (None, None)
        };

        Ok(FormDataVO {
            id: row.id,
            form_version: row.form_version,
            data_json: row.data_json.clone(),
            form_json,
            options_json,
            create_by: row.create_by.map(|v| v.to_string()),
            create_by_name,
            create_time: row.create_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        })
    }

    /// 确保默认挂载目录"表单中心"存在，不存在则创建
    async fn ensure_default_catalog(&self) -> AppResult<entity::sys_menu::Model> {
        let existing = entity::sys_menu::Entity::find()
            .filter(entity::sys_menu::Column::Name.eq(DEFAULT_CATALOG_NAME))
            .filter(entity::sys_menu::Column::Type.eq("C"))
            .one(self.db)
            .await?;
        if let Some(catalog) = existing {
            return Ok(catalog);
        }

        let admin_catalog = entity::sys_menu::Entity::find()
            .filter(entity::sys_menu::Column::ParentId.eq(0))
            .filter(entity::sys_menu::Column::Name.eq(FORM_ADMIN_CATALOG_NAME))
            .filter(entity::sys_menu::Column::Type.eq("C"))
            .one(self.db)
            .await?;
        let under_admin = admin_catalog.is_some();
        let parent_id = admin_catalog.as_ref().map_or(0, |c| c.id);
        let tree_path = if parent_id == 0 {
            "0".to_string()
        } else {
            format!("{},{}", admin_catalog.and_then(|c| c.tree_path).unwrap_or_default(), parent_id)
        };
        let now = Utc::now().naive_utc();

        entity::sys_menu::ActiveModel {
            parent_id: Set(parent_id),
            name: Set(DEFAULT_CATALOG_NAME.to_string()),
            r#type: Set("C".to_string()),
            route_path: Set(Some(if under_admin {
                "center".to_string()
            } else {
                "/form-center".to_string()
            })),
            component: Set(Some("Layout".to_string())),
            visible: Set(Some(1)),
            sort: Set(Some(5)),
            icon: Set(Some("el-icon-EditPen".to_string())),
            tree_path: Set(Some(tree_path)),
            create_time: Set(Some(now)),
            update_time: Set(Some(now)),
            ..Default::default()
        }
        .insert(self.db)
        .await?;

        entity::sys_menu::Entity::find()
            .filter(entity::sys_menu::Column::Name.eq(DEFAULT_CATALOG_NAME))
            .filter(entity::sys_menu::Column::Type.eq("C"))
            .one(self.db)
            .await?
            .ok_or_else(|| bad("默认目录创建失败"))
    }

    /// 追加式角色授权：向角色授予菜单及 tree_path 祖先目录
    async fn assign_menu_to_roles(&self, menu_id: i64, role_ids: &[i64]) -> AppResult<()> {
        if role_ids.is_empty() {
            return Ok(());
        }

        let Some(menu) = entity::sys_menu::Entity::find_by_id(menu_id).one(self.db).await? else {
            return Ok(());
        };

        let mut menu_ids: Vec<i64> = vec![menu_id];
        if let Some(tree_path) = menu.tree_path.as_deref() {
            for part in tree_path.split(',') {
                let part = part.trim();
                if part != "0" && !part.is_empty() {
                    if let Ok(id) = part.parse::<i64>() {
                        if !menu_ids.contains(&id) {
                            menu_ids.push(id);
                        }
                    }
                }
            }
        }

        let existing = entity::sys_role_menu::Entity::find()
            .filter(entity::sys_role_menu::Column::MenuId.is_in(menu_ids.clone()))
            .filter(entity::sys_role_menu::Column::RoleId.is_in(role_ids.to_vec()))
            .all(self.db)
            .await?;

        for role_id in role_ids {
            for grant_menu_id in &menu_ids {
                let exists = existing
                    .iter()
                    .any(|r| r.role_id == *role_id && r.menu_id == *grant_menu_id);
                if !exists {
                    entity::sys_role_menu::ActiveModel {
                        role_id: Set(*role_id),
                        menu_id: Set(*grant_menu_id),
                    }
                    .insert(self.db)
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// 公开表单按 formKey + IP 限流（固定窗口计数，Redis 异常时放行）
    async fn check_public_submit_limit(&self, form_key: &str, ip: &str) -> bool {
        let key = format!("form:public:submit:{form_key}:{ip}");
        let mut conn = self.redis.clone();
        let count: Result<i64, _> = redis::cmd("INCR").arg(&key).query_async(&mut conn).await;
        match count {
            Ok(n) => {
                if n == 1 {
                    let _: Result<i64, _> = redis::cmd("EXPIRE").arg(&key).arg(60).query_async(&mut conn).await;
                }
                n <= PUBLIC_SUBMIT_LIMIT
            }
            Err(e) => {
                tracing::warn!("公开表单限流 Redis 异常，放行 formKey={form_key}: {e}");
                true
            }
        }
    }
}

fn bad(msg: &str) -> AppError {
    AppError::Business(ResultCode::UserRequestParameterError, Some(msg.into()))
}

fn parse_ids(ids: &str) -> Vec<i64> {
    ids.split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

fn parse_json_or_empty_array(text: Option<&str>) -> Value {
    text.and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| json!([]))
}

fn to_definition_vo(m: &entity::form_definition::Model) -> FormDefinitionVO {
    FormDefinitionVO {
        id: m.id,
        form_key: m.form_key.clone(),
        form_name: m.form_name.clone(),
        description: m.description.clone(),
        status: m.status,
        version: m.version,
        is_public: m.is_public,
        category: m.category.clone(),
        create_time: m.create_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
    }
}

fn to_render_vo(m: &entity::form_definition::Model) -> FormRenderVO {
    FormRenderVO {
        form_key: m.form_key.clone(),
        form_name: m.form_name.clone(),
        version: m.version,
        form_json: m.form_json.clone(),
        options_json: m.options_json.clone(),
    }
}

/// 子菜单树路径 = 父目录 tree_path + 父目录ID
fn catalog_tree_path(parent: &entity::sys_menu::Model) -> String {
    format!(
        "{},{}",
        parent.tree_path.clone().unwrap_or_default(),
        parent.id
    )
}

/// 表单标识转大驼峰，用于生成全局唯一的路由名称
fn to_upper_camel(value: &str) -> String {
    value
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// 校验表单规则结构（AI 生成或外部导入的规则先过这一层）
pub fn validate_rule(form_json: &str) -> AppResult<String> {
    let rules: Value = serde_json::from_str(form_json).map_err(|_| bad("表单规则解析失败"))?;
    let array = rules
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or_else(|| bad("表单规则格式非法：应为非空数组"))?;
    check_nodes(array)?;
    Ok(form_json.to_string())
}

fn check_nodes(nodes: &[Value]) -> AppResult<()> {
    for node in nodes {
        let Some(object) = node.as_object() else {
            return Err(bad("表单规则格式非法：节点必须为对象"));
        };
        if !object.get("type").is_some_and(Value::is_string) {
            return Err(bad("表单规则格式非法：节点缺少 type"));
        }
        let children = object.get("children").and_then(Value::as_array);
        match children {
            Some(children) if !children.is_empty() => check_nodes(children)?,
            _ => {
                if !object.get("field").is_some_and(Value::is_string) {
                    return Err(bad("表单规则格式非法：字段节点缺少 field"));
                }
            }
        }
    }
    Ok(())
}

/// 校验并过滤表单数据：仅保留 rule 中声明 field 的数据项，逐字段执行必填与类型校验
fn validate_and_filter(form_json: &Value, data: &Value) -> AppResult<Value> {
    let Some(rules) = form_json.as_array() else {
        return Err(bad("表单规则格式非法"));
    };

    let mut filtered = Map::new();
    let mut errors: Vec<String> = Vec::new();
    walk_rules(rules, data, &mut filtered, &mut errors);
    if !errors.is_empty() {
        return Err(bad(&errors.join("；")));
    }
    Ok(Value::Object(filtered))
}

fn walk_rules(
    nodes: &[Value],
    data: &Value,
    filtered: &mut Map<String, Value>,
    errors: &mut Vec<String>,
) {
    for node in nodes {
        let Some(object) = node.as_object() else {
            continue;
        };

        if let Some(field) = object.get("field").and_then(Value::as_str).filter(|f| !f.is_empty()) {
            // 字段白名单：rule 未声明的 field 直接丢弃
            if let Some(value) = data.get(field) {
                filtered.insert(field.to_string(), value.clone());
            }
            validate_field(object, field, data, errors);
        }

        if let Some(children) = object.get("children").and_then(Value::as_array) {
            walk_rules(children, data, filtered, errors);
        }
    }
}

fn validate_field(
    rule: &Map<String, Value>,
    field: &str,
    data: &Value,
    errors: &mut Vec<String>,
) {
    let Some(validates) = rule.get("validate").and_then(Value::as_array) else {
        return;
    };

    let title = rule
        .get("title")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .unwrap_or(field);
    let value = data.get(field);
    let empty = value.is_none_or(is_empty_value);

    for validate in validates {
        let Some(validate) = validate.as_object() else {
            continue;
        };

        // 必填：null/空串/空集合均视为未填写
        if validate.get("required").and_then(Value::as_bool) == Some(true) && empty {
            let message = validate
                .get("message")
                .and_then(Value::as_str)
                .filter(|m| !m.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("「{title}」不能为空"));
            errors.push(message);
        }

        // 类型：仅值非空时校验，避免与 required 错误重复
        if !empty {
            if let (Some(value), Some(kind)) = (value, validate.get("type").and_then(Value::as_str)) {
                if !check_type(kind, value) {
                    errors.push(format!("「{title}」数据类型非法"));
                }
            }
        }
    }
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

fn check_type(kind: &str, value: &Value) -> bool {
    match kind {
        "number" | "integer" | "float" => {
            value.is_number() || value.as_str().is_some_and(|s| s.trim().parse::<f64>().is_ok())
        }
        "array" => value.is_array(),
        _ => true,
    }
}
