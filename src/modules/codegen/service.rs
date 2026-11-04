//! 代码生成服务：PostgreSQL 元数据 + 模板渲染 + 配置持久化 + zip 打包

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement,
};
use serde::Serialize;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::entity::{gen_table, gen_table_column};
use crate::error::{AppError, AppResult};
use super::dto::{ColumnVO, GenConfigForm, GenConfigVO, PreviewVO};
use super::templates;
use crate::modules::system::menu::service::MenuService;
use crate::result_code::ResultCode;
use crate::util::{kebab_case, plural_kebab, snake_case};

// ── 模板上下文 ──

/// 单个字段的模板上下文（用于前端模板渲染）
#[derive(Debug, Clone, Serialize)]
pub struct FieldMeta {
    pub column_name: String,
    pub column_pascal: String,
    pub rust_field_name: String,
    pub column_type: String,
    pub field_name: String,
    pub default_field_name: String,
    pub field_type: String,
    pub field_comment: String,
    pub ts_type: String,
    pub is_show_in_list: bool,
    pub is_show_in_form: bool,
    pub is_show_in_query: bool,
    pub is_required: bool,
    pub form_type_name: String,   // "INPUT", "SELECT", etc.
    pub query_type_name: String,  // "EQ", "LIKE", etc.
    pub dict_type: String,
    pub max_length: Option<i32>,
    pub is_pk: bool,
    pub is_audit: bool,
    pub is_editable: bool,
    pub is_nullable: bool,
    pub is_string: bool,
    pub is_datetime: bool,
    pub is_integer: bool,
    pub is_bool: bool,
    pub rust_type: String,
}

/// 整表模板上下文
#[derive(Debug, Clone, Serialize)]
pub struct TableMeta {
    pub table_name: String,
    pub entity_name: String,
    pub entity_lower: String,
    pub module_name: String,
    pub entity_kebab: String,
    /// 接口路径复数形式（如 sys-dicts），路由、前端 BASE_URL 与 Swagger 注解共用
    pub entity_kebab_plural: String,
    /// 权限标识前缀（如 sys:article），路由守卫、前端 v-hasPerm 与菜单按钮权限共用
    pub permission_prefix: String,
    pub entity_upper_snake: String,
    pub pk_name: String,
    pub pk_field_name: String,
    pub pk_column_pascal: String,
    pub business_name: String,
    pub entity_comment: String,
    pub package_name: String,
    pub author: String,
    pub columns: Vec<FieldMeta>,
    pub has_datetime: bool,
    pub has_json: bool,
    pub has_uuid: bool,
    pub has_decimal: bool,
    pub has_required_field: bool,
    pub has_soft_delete: bool,
    pub has_create_time: bool,
    pub has_update_time: bool,
    pub has_create_by: bool,
    pub has_update_by: bool,
    pub create_time_nullable: bool,
    pub update_time_nullable: bool,
    pub create_by_nullable: bool,
    pub update_by_nullable: bool,
}

/// 数据库原始列信息
struct RawColumn {
    name: String,
    udt: String,
    data_type: String,
    is_nullable: bool,
    comment: String,
    is_pk: bool,
}

/// 代码生成提示词路径
const CODEGEN_SYSTEM_PROMPT_PATH: &str = "codegen/system.md";

/// 未填写模块名时的默认模块（决定前端目录与菜单 component 前缀）
const DEFAULT_MODULE_NAME: &str = "system";

/// 未填写表前缀时的默认剥离前缀
const DEFAULT_REMOVE_TABLE_PREFIX: &str = "sys_";

/// 模块名 → 权限前缀，未配置时用模块名
const PERMISSION_NAMESPACES: [(&str, &str); 1] = [("system", "sys")];

/// 构造用户提示词，附带表结构与字段清单
fn build_ai_user_prompt(config: &GenConfigVO, requirement: Option<&str>) -> String {
    let columns = config
        .field_configs
        .iter()
        .map(|field| {
            serde_json::json!({
                "columnName": field.column_name,
                "columnType": field.data_type,
                "fieldComment": field.column_comment,
                "isRequired": field.is_required,
            })
        })
        .collect::<Vec<_>>();

    format!(
        "表名：{}\n当前业务名：{}\n补充说明：{}\n字段列表：\n{}",
        config.table_name,
        config.business_name.as_deref().unwrap_or(""),
        requirement
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("无"),
        serde_json::to_string(&columns).unwrap_or_else(|_| "[]".to_string())
    )
}

/// 按列名匹配回填字段配置
fn apply_ai_field_configs(fields: &mut [ColumnVO], value: &serde_json::Value) {
    let Some(ai_fields) = value["fieldConfigs"].as_array() else {
        return;
    };

    for field in fields.iter_mut() {
        let column_name = field.column_name.clone();
        let Some(ai_field) = ai_fields
            .iter()
            .find(|item| item["columnName"].as_str().map(str::trim) == Some(column_name.as_str()))
        else {
            continue;
        };

        if let Some(comment) = ai_field["fieldComment"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            field.column_comment = comment.to_string();
        }
        if let Some(dict_type) = ai_field["dictType"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            field.dict_type = Some(dict_type.to_string());
        }
        if let Some(form_type) = form_type_value(ai_field["formType"].as_str().unwrap_or_default()) {
            field.form_type = Some(form_type);
        }
        if let Some(query_type) = query_type_value(ai_field["queryType"].as_str().unwrap_or_default()) {
            field.query_type = Some(query_type);
        }
        if let Some(ai_value) = ai_field["isRequired"].as_i64() {
            field.is_required = ai_value as i16;
        }
        if let Some(ai_value) = ai_field["isShowInList"].as_i64() {
            field.is_show_in_list = ai_value as i16;
        }
        if let Some(ai_value) = ai_field["isShowInForm"].as_i64() {
            field.is_show_in_form = ai_value as i16;
        }
        if let Some(ai_value) = ai_field["isShowInQuery"].as_i64() {
            field.is_show_in_query = ai_value as i16;
        }
    }
}

/// 表单类型枚举名转存储值
fn form_type_value(name: &str) -> Option<i16> {
    match name.trim().to_uppercase().as_str() {
        "INPUT" => Some(1),
        "SELECT" => Some(2),
        "RADIO" => Some(3),
        "CHECK_BOX" => Some(4),
        "INPUT_NUMBER" => Some(5),
        "SWITCH" => Some(6),
        "TEXT_AREA" => Some(7),
        "DATE" => Some(8),
        "DATE_TIME" => Some(9),
        "HIDDEN" => Some(10),
        _ => None,
    }
}

/// 查询类型枚举名转存储值
fn query_type_value(name: &str) -> Option<i16> {
    match name.trim().to_uppercase().as_str() {
        "EQ" => Some(1),
        "LIKE" => Some(2),
        "IN" => Some(3),
        "BETWEEN" => Some(4),
        "GT" => Some(5),
        "GE" => Some(6),
        "LT" => Some(7),
        "LE" => Some(8),
        "NE" => Some(9),
        "LIKE_LEFT" => Some(10),
        _ => None,
    }
}

// ── 服务 ──

pub struct CodegenService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> CodegenService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    // 表列表

    /// 读取当前 schema 下所有基表（排除代码生成器自身表与 flyway 历史表）
    pub async fn list_tables(&self) -> AppResult<Vec<(String, String)>> {
        let sql = r#"
            SELECT t.table_name,
                   obj_description(('"' || t.table_name || '"')::regclass) AS table_comment
            FROM information_schema.tables t
            WHERE t.table_schema = current_schema()
              AND t.table_type = 'BASE TABLE'
              AND t.table_name NOT IN ('gen_table', 'gen_table_column', 'flyway_schema_history')
            ORDER BY t.table_name
        "#;
        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                Vec::<sea_orm::Value>::new(),
            ))
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let table_name: String = row.try_get("", "table_name")?;
            let table_comment: Option<String> = row.try_get("", "table_comment")?;
            list.push((table_name, table_comment.unwrap_or_default()));
        }
        Ok(list)
    }

    // 元数据查询

    /// 读取表字段元数据
    async fn get_columns(&self, table_name: &str) -> AppResult<Vec<RawColumn>> {
        let sql = r#"
            SELECT
                c.column_name,
                c.udt_name,
                c.data_type,
                c.is_nullable,
                pg_catalog.col_description(
                    ('"' || c.table_schema || '"."' || c.table_name || '"')::regclass::oid,
                    c.ordinal_position
                ) AS column_comment,
                CASE WHEN pk.column_name IS NOT NULL THEN 'PRI' ELSE '' END AS column_key
            FROM information_schema.columns c
            LEFT JOIN (
                SELECT kcu.column_name
                FROM information_schema.table_constraints tc
                JOIN information_schema.key_column_usage kcu
                  ON tc.constraint_name = kcu.constraint_name
                 AND tc.table_schema = kcu.table_schema
                 AND tc.table_name = kcu.table_name
                WHERE tc.constraint_type = 'PRIMARY KEY'
                  AND tc.table_name = $1
                  AND tc.table_schema = current_schema()
            ) pk ON c.column_name = pk.column_name
            WHERE c.table_name = $1 AND c.table_schema = current_schema()
            ORDER BY c.ordinal_position
        "#;
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [table_name.to_string().into()],
        );
        let rows = self.db.query_all(stmt).await?;
        let mut cols = Vec::with_capacity(rows.len());
        for row in &rows {
            let name: String = row.try_get("", "column_name")?;
            let udt: String = row.try_get("", "udt_name")?;
            let data_type: String = row.try_get("", "data_type")?;
            let is_nullable: String = row.try_get("", "is_nullable")?;
            let comment: Option<String> = row.try_get("", "column_comment")?;
            let column_key: String = row.try_get("", "column_key")?;
            cols.push(RawColumn {
                name,
                udt,
                data_type,
                is_nullable: is_nullable.eq_ignore_ascii_case("YES"),
                comment: comment.unwrap_or_default(),
                is_pk: column_key == "PRI",
            });
        }
        Ok(cols)
    }

    /// 从元数据构造 TableMeta（无 DB 配置时的默认推导）
    async fn load_meta_from_db_schema(&self, table_name: &str) -> AppResult<TableMeta> {
        let raw = Self::ensure_table_exists(self.get_columns(table_name).await?, table_name)?;

        // 实体名先去表前缀再转大驼峰，否则 sys_article 推导成 SysArticle，权限串多出一层模块名
        let entity_name = default_entity_name(table_name, DEFAULT_REMOVE_TABLE_PREFIX);
        let entity_lower = snake_case(&entity_name);
        let module_name = DEFAULT_MODULE_NAME.to_string();
        let entity_kebab = kebab_case(&entity_name);
        let entity_upper_snake = entity_lower.to_uppercase();
        let pk_name = raw
            .iter()
            .find(|c| c.is_pk)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "id".to_string());

        let mut has_datetime = false;
        let mut has_json = false;
        let mut has_uuid = false;
        let mut has_decimal = false;

        let columns: Vec<FieldMeta> = raw
            .iter()
            .map(|c| {
                let rust_type = pg_to_rust(&c.udt, &c.data_type).to_string();
                let ts_type = rust_to_ts(&rust_type);
                let is_string = matches!(
                    c.udt.as_str(),
                    "text" | "varchar" | "char" | "bpchar" | "name" | "citext"
                );
                let is_datetime = matches!(
                    c.udt.as_str(),
                    "timestamp" | "timestamptz" | "date" | "time"
                );
                let is_integer = matches!(
                    c.udt.as_str(),
                    "int2" | "int4" | "int8" | "serial" | "bigserial" | "smallserial"
                );
                let is_bool = c.udt == "bool";
                let is_uuid_field = c.udt == "uuid";
                let is_decimal_field = matches!(c.udt.as_str(), "numeric" | "decimal");
                has_datetime |= is_datetime;
                has_json |= matches!(c.udt.as_str(), "json" | "jsonb");
                has_uuid |= is_uuid_field;
                has_decimal |= is_decimal_field;

                let default_field_name = camel_case(&c.name);
                let is_audit = matches!(
                    c.name.as_str(),
                    "create_time" | "update_time" | "create_by" | "update_by" | "is_deleted"
                );
                let is_editable = !c.is_pk && !is_audit;
                let is_queryable = is_editable && (is_string || is_integer || is_bool);

                FieldMeta {
                    column_name: c.name.clone(),
                    column_pascal: pascal_case(&c.name),
                    rust_field_name: rust_ident(&c.name),
                    column_type: format!("{}({})", c.udt, ""),
                    field_name: default_field_name.clone(),
                    default_field_name,
                    field_type: rust_type.clone(),
                    field_comment: c.comment.clone(),
                    ts_type: ts_type.to_string(),
                    is_show_in_list: is_editable,
                    is_show_in_form: is_editable,
                    is_show_in_query: is_queryable,
                    is_required: !c.is_nullable && is_editable,
                    form_type_name: default_form_type(&rust_type, &c.name),
                    query_type_name: default_query_type(&rust_type),
                    dict_type: String::new(),
                    max_length: None,
                    is_pk: c.is_pk,
                    is_audit,
                    is_editable,
                    is_nullable: c.is_nullable,
                    is_string,
                    is_datetime,
                    is_integer,
                    is_bool,
                    rust_type,
                }
            })
            .collect();

        let has_required_field = columns.iter().any(|c| c.is_required);
        let audit_column = |name: &str| columns.iter().find(|c| c.column_name == name);
        let has_soft_delete = audit_column("is_deleted").is_some();
        let has_create_time = audit_column("create_time").is_some();
        let has_update_time = audit_column("update_time").is_some();
        let has_create_by = audit_column("create_by").is_some();
        let has_update_by = audit_column("update_by").is_some();
        let create_time_nullable = audit_column("create_time").is_some_and(|c| c.is_nullable);
        let update_time_nullable = audit_column("update_time").is_some_and(|c| c.is_nullable);
        let create_by_nullable = audit_column("create_by").is_some_and(|c| c.is_nullable);
        let update_by_nullable = audit_column("update_by").is_some_and(|c| c.is_nullable);

        let permission_prefix = resolve_permission_prefix(&module_name, &entity_name);

        Ok(TableMeta {
            table_name: table_name.to_string(),
            entity_name: entity_name.clone(),
            entity_lower,
            module_name,
            entity_kebab,
            entity_kebab_plural: plural_kebab(&entity_name),
            permission_prefix,
            entity_upper_snake,
            pk_field_name: rust_ident(&pk_name),
            pk_column_pascal: pascal_case(&pk_name),
            pk_name,
            business_name: entity_name.clone(),
            entity_comment: entity_name,
            package_name: "com.youlai.rust".into(),
            author: "youlai-axum".into(),
            columns,
            has_datetime,
            has_json,
            has_uuid,
            has_decimal,
            has_required_field,
            has_soft_delete,
            has_create_time,
            has_update_time,
            has_create_by,
            has_update_by,
            create_time_nullable,
            update_time_nullable,
            create_by_nullable,
            update_by_nullable,
        })
    }

    // 配置 CRUD

    /// 用 AI 推断代码生成配置，未推断出的字段沿用原有默认值
    pub async fn ai_fill_config(
        &self,
        http: &reqwest::Client,
        ai_config: &crate::config::AiConfig,
        table_name: &str,
        requirement: Option<&str>,
    ) -> AppResult<GenConfigVO> {
        let mut config = self.config(table_name).await?;
        let system_prompt = crate::ai::prompt(CODEGEN_SYSTEM_PROMPT_PATH)?;
        let user_prompt = build_ai_user_prompt(&config, requirement);
        let value = crate::ai::chat_json_list(http, ai_config, system_prompt, &user_prompt).await?;

        if let Some(name) = value["businessName"]
            .as_str()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            config.business_name = Some(name.to_string());
        }
        apply_ai_field_configs(&mut config.field_configs, &value);

        Ok(config)
    }

    /// 获取代码生成配置（优先从 DB，回退到元数据推导）
    pub async fn config(&self, table_name: &str) -> AppResult<GenConfigVO> {
        // 尝试从 gen_table 读取
        let table_model = gen_table::Entity::find()
            .filter(gen_table::Column::TableName.eq(table_name))
            .one(self.db)
            .await?;

        match table_model {
            Some(t) => {
                let columns = gen_table_column::Entity::find()
                    .filter(gen_table_column::Column::TableId.eq(t.id))
                    .order_by_asc(gen_table_column::Column::FieldSort)
                    .all(self.db)
                    .await?;

                let meta = self.load_meta_from_db_schema(table_name).await?;

                let field_configs: Vec<ColumnVO> = meta
                    .columns
                    .iter()
                    .map(|mc| {
                        let saved = columns.iter().find(|sc| {
                            sc.column_name.as_deref() == Some(mc.column_name.as_str())
                        });
                        let field_type = saved
                            .and_then(|s| s.field_type.clone())
                            .unwrap_or_else(|| mc.field_type.clone());
                        let ts_type = rust_to_ts(&field_type).to_string();
                        ColumnVO {
                            column_name: mc.column_name.clone(),
                            data_type: mc.column_type.clone(),
                            column_comment: mc.field_comment.clone(),
                            is_nullable: if mc.is_nullable { "YES".into() } else { "NO".into() },
                            is_pk: mc.is_pk,
                            column_key: if mc.is_pk { "PRI".into() } else { String::new() },
                            field_name: saved
                                .map(|s| s.field_name.clone())
                                .unwrap_or_else(|| mc.field_name.clone()),
                            field_type,
                            ts_type,
                            is_show_in_list: saved.map(|s| s.is_show_in_list).unwrap_or(if mc.is_show_in_list { 1 } else { 0 }),
                            is_show_in_form: saved.map(|s| s.is_show_in_form).unwrap_or(if mc.is_show_in_form { 1 } else { 0 }),
                            is_show_in_query: saved.map(|s| s.is_show_in_query).unwrap_or(if mc.is_show_in_query { 1 } else { 0 }),
                            is_required: saved.and_then(|s| s.is_required).unwrap_or(if mc.is_required { 1 } else { 0 }),
                            form_type: saved.and_then(|s| s.form_type),
                            query_type: saved.and_then(|s| s.query_type),
                            dict_type: saved.and_then(|s| s.dict_type.clone()),
                            max_length: saved.and_then(|s| s.max_length),
                            field_sort: saved.and_then(|s| s.field_sort),
                        }
                    })
                    .collect();

                Ok(GenConfigVO {
                    id: Some(t.id),
                    table_name: t.table_name,
                    business_name: Some(t.business_name),
                    module_name: t.module_name,
                    package_name: Some(t.package_name),
                    entity_name: t.entity_name,
                    author: Some(t.author),
                    parent_menu_id: t.parent_menu_id,
                    page_type: t.page_type,
                    remove_table_prefix: t.remove_table_prefix,
                    field_configs,
                })
            }
            None => {
                let meta = self.load_meta_from_db_schema(table_name).await?;
                let field_configs: Vec<ColumnVO> = meta
                    .columns
                    .iter()
                    .map(|mc| ColumnVO {
                        column_name: mc.column_name.clone(),
                        data_type: mc.column_type.clone(),
                        column_comment: mc.field_comment.clone(),
                        is_nullable: if mc.is_nullable { "YES".into() } else { "NO".into() },
                        is_pk: mc.is_pk,
                        column_key: if mc.is_pk { "PRI".into() } else { String::new() },
                        field_name: mc.field_name.clone(),
                        field_type: mc.field_type.clone(),
                        ts_type: mc.ts_type.clone(),
                        is_show_in_list: if mc.is_show_in_list { 1 } else { 0 },
                        is_show_in_form: if mc.is_show_in_form { 1 } else { 0 },
                        is_show_in_query: if mc.is_show_in_query { 1 } else { 0 },
                        is_required: if mc.is_required { 1 } else { 0 },
                        form_type: Some(form_type_name_to_int(&mc.form_type_name)),
                        query_type: Some(query_type_name_to_int(&mc.query_type_name)),
                        dict_type: Some(mc.dict_type.clone()),
                        max_length: mc.max_length,
                        field_sort: None,
                    })
                    .collect();

                Ok(GenConfigVO {
                    id: None,
                    table_name: meta.table_name,
                    business_name: Some(meta.business_name),
                    module_name: Some(meta.module_name),
                    package_name: Some(meta.package_name),
                    entity_name: meta.entity_name,
                    author: Some(meta.author),
                    parent_menu_id: None,
                    page_type: Some("classic".into()),
                    remove_table_prefix: None,
                    field_configs,
                })
            }
        }
    }

    /// 保存代码生成配置（upsert gen_table，全量替换 gen_table_column）
    pub async fn save_config(&self, table_name: &str, form: &GenConfigForm) -> AppResult<()> {
        // 查找已有记录
        let existing = gen_table::Entity::find()
            .filter(gen_table::Column::TableName.eq(table_name))
            .one(self.db)
            .await?;

        // 新建行的实体名兜底走去前缀推导，否则 sys_article 会生成 SysArticle，权限串跟着变成 sys:sys-article
        let remove_prefix = form
            .remove_table_prefix
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(DEFAULT_REMOVE_TABLE_PREFIX);

        let table_id: i64 = if let Some(t) = existing {
            let mut active: gen_table::ActiveModel = t.into();
            if let Some(ref v) = form.business_name { active.business_name = Set(v.clone()); }
            if let Some(ref v) = form.module_name { active.module_name = Set(Some(v.clone())); }
            if let Some(ref v) = form.package_name { active.package_name = Set(v.clone()); }
            if let Some(ref v) = form.entity_name { active.entity_name = Set(v.clone()); }
            if let Some(ref v) = form.author { active.author = Set(v.clone()); }
            if let Some(v) = form.parent_menu_id { active.parent_menu_id = Set(Some(v)); }
            if let Some(ref v) = form.page_type { active.page_type = Set(Some(v.clone())); }
            if let Some(ref v) = form.remove_table_prefix { active.remove_table_prefix = Set(Some(v.clone())); }
            let id = active.id.clone().unwrap();
            active.update(self.db).await?;
            id
        } else {
            // 新建
            let active = gen_table::ActiveModel {
                table_name: Set(table_name.to_string()),
                module_name: Set(form.module_name.clone()),
                package_name: Set(form.package_name.clone().unwrap_or_else(|| "com.youlai.rust".into())),
                business_name: Set(form.business_name.clone().unwrap_or_else(|| table_name.to_string())),
                entity_name: Set(
                    form.entity_name
                        .clone()
                        .unwrap_or_else(|| default_entity_name(table_name, remove_prefix)),
                ),
                author: Set(form.author.clone().unwrap_or_else(|| "youlai-axum".into())),
                parent_menu_id: Set(form.parent_menu_id),
                remove_table_prefix: Set(form.remove_table_prefix.clone()),
                page_type: Set(form.page_type.clone()),
                ..Default::default()
            };
            let inserted = active.insert(self.db).await?;
            inserted.id
        };

        gen_table_column::Entity::delete_many()
            .filter(gen_table_column::Column::TableId.eq(table_id))
            .exec(self.db)
            .await?;

        // 插入新字段配置
        if let Some(ref field_configs) = form.field_configs {
            for (idx, fc) in field_configs.iter().enumerate() {
                let col_active = gen_table_column::ActiveModel {
                    table_id: Set(table_id),
                    column_name: Set(fc.column_name.clone()),
                    column_type: Set(fc.column_type.clone()),
                    field_name: Set(fc.field_name.clone().unwrap_or_default()),
                    field_type: Set(fc.field_type.clone()),
                    field_sort: Set(Some(fc.field_sort.unwrap_or(idx as i32))),
                    field_comment: Set(fc.field_comment.clone()),
                    is_show_in_list: Set(fc.is_show_in_list.unwrap_or(0)),
                    is_show_in_form: Set(fc.is_show_in_form.unwrap_or(0)),
                    is_show_in_query: Set(fc.is_show_in_query.unwrap_or(0)),
                    is_required: Set(fc.is_required),
                    max_length: Set(fc.max_length),
                    form_type: Set(fc.form_type),
                    query_type: Set(fc.query_type),
                    dict_type: Set(fc.dict_type.clone()),
                    ..Default::default()
                };
                col_active.insert(self.db).await?;
            }
        }

        // 选了上级菜单就同步生成页面菜单与增删改查按钮，权限前缀与生成代码的守卫同源；
        // 取值以落库后的配置为准，重复保存按路由名去重，不会挂出多份菜单
        if let Some(parent_menu_id) = form.parent_menu_id.filter(|id| *id > 0) {
            if let Some(saved) = gen_table::Entity::find_by_id(table_id).one(self.db).await? {
                let module_name = saved
                    .module_name
                    .as_deref()
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or(DEFAULT_MODULE_NAME);
                let business_name = saved.business_name.trim();
                let business_name = if business_name.is_empty() {
                    table_name
                } else {
                    business_name
                };
                MenuService::new(self.db)
                    .add_menu_for_codegen(
                        parent_menu_id,
                        module_name,
                        business_name,
                        &saved.entity_name,
                        &resolve_permission_prefix(module_name, &saved.entity_name),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// 删除代码生成配置
    pub async fn delete_config(&self, table_name: &str) -> AppResult<()> {
        let table_model = gen_table::Entity::find()
            .filter(gen_table::Column::TableName.eq(table_name))
            .one(self.db)
            .await?;

        if let Some(t) = table_model {
            gen_table_column::Entity::delete_many()
                .filter(gen_table_column::Column::TableId.eq(t.id))
                .exec(self.db)
                .await?;
            gen_table::Entity::delete_by_id(t.id).exec(self.db).await?;
        }
        Ok(())
    }

    // 预览与下载

    /// 预览生成的代码
    pub async fn preview(
        &self,
        table_name: &str,
        page_type: &str,
        frontend_type: &str,
    ) -> AppResult<Vec<PreviewVO>> {
        let meta = self.build_template_meta(table_name, page_type).await?;
        let files = templates::render(&meta, page_type, frontend_type)?;

        let vos = files
            .into_iter()
            .map(|f| {
                let ext = f.file_name.rsplit('.').next().unwrap_or("");
                let language = if ext == "vue" { "vue".into() } else { ext.to_string() };
                PreviewVO {
                    path: f.path,
                    file_name: f.file_name,
                    content: f.content,
                    scope: f.scope,
                    language,
                }
            })
            .collect();
        Ok(vos)
    }

    /// 下载生成的代码（zip）
    pub async fn download(
        &self,
        table_name: &str,
        page_type: &str,
        frontend_type: &str,
    ) -> AppResult<Vec<u8>> {
        let meta = self.build_template_meta(table_name, page_type).await?;
        let files = templates::render(&meta, page_type, frontend_type)?;

        let mut cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for f in &files {
                let entry = format!("{}/{}", f.path, f.file_name);
                zip.start_file(&entry, opts).map_err(zip_err)?;
                zip.write_all(f.content.as_bytes()).map_err(|e| {
                    AppError::Business(ResultCode::SystemError, Some(e.to_string()))
                })?;
            }
            zip.finish().map_err(zip_err)?;
        }
        Ok(cursor.into_inner())
    }

    /// 构造模板上下文：合并 DB 配置与数据库元数据
    async fn build_template_meta(&self, table_name: &str, _page_type: &str) -> AppResult<TableMeta> {
        let mut meta = self.load_meta_from_db_schema(table_name).await?;

        if let Some(t) = gen_table::Entity::find()
            .filter(gen_table::Column::TableName.eq(table_name))
            .one(self.db)
            .await?
        {
            meta.entity_name = t.entity_name;
            meta.module_name = t.module_name.unwrap_or_else(|| meta.module_name.clone());
            meta.package_name = t.package_name;
            meta.author = t.author;
            meta.business_name = t.business_name;
            meta.entity_comment = meta.business_name.clone();

            let saved_columns = gen_table_column::Entity::find()
                .filter(gen_table_column::Column::TableId.eq(t.id))
                .order_by_asc(gen_table_column::Column::FieldSort)
                .all(self.db)
                .await?;

            for mc in &mut meta.columns {
                if let Some(sc) = saved_columns.iter().find(|sc| {
                    sc.column_name.as_deref() == Some(mc.column_name.as_str())
                }) {
                    let saved_name = sc.field_name.trim();
                    if !saved_name.is_empty() {
                        mc.field_name = camel_case(saved_name);
                    }
                    let ft = sc.field_type.clone().unwrap_or_else(|| mc.field_type.clone());
                    mc.field_type = ft.clone();
                    mc.is_show_in_list = sc.is_show_in_list != 0;
                    mc.is_show_in_form = sc.is_show_in_form != 0;
                    mc.is_show_in_query = sc.is_show_in_query != 0
                        && (mc.is_string || mc.is_integer || mc.is_bool);
                    mc.is_required = sc.is_required.unwrap_or(0) != 0;
                    mc.form_type_name = int_to_form_type_name(sc.form_type.unwrap_or(0));
                    mc.query_type_name = int_to_query_type_name(sc.query_type.unwrap_or(0));
                    mc.dict_type = sc.dict_type.clone().unwrap_or_default();
                    mc.max_length = sc.max_length;
                    mc.ts_type = rust_to_ts(&ft).to_string();
                }
            }
            meta.has_required_field = meta.columns.iter().any(|c| c.is_required);
        }

        meta.entity_lower = snake_case(&meta.entity_name);
        meta.entity_kebab = kebab_case(&meta.entity_name);
        meta.entity_kebab_plural = plural_kebab(&meta.entity_name);
        meta.entity_upper_snake = meta.entity_lower.to_uppercase();
        meta.entity_comment = meta.business_name.clone();
        meta.permission_prefix = resolve_permission_prefix(&meta.module_name, &meta.entity_name);

        Ok(meta)
    }

    fn ensure_table_exists(raw: Vec<RawColumn>, table_name: &str) -> AppResult<Vec<RawColumn>> {
        if raw.is_empty() {
            return Err(AppError::Business(
                ResultCode::UserRequestParameterError,
                Some(format!("表 {table_name} 不存在或无字段")),
            ));
        }
        Ok(raw)
    }
}

// 工具函数

fn zip_err(e: zip::result::ZipError) -> AppError {
    AppError::Business(ResultCode::SystemError, Some(e.to_string()))
}

/// 按表名推导实体名：先去表前缀再转大驼峰（sys_article → Article）
fn default_entity_name(table_name: &str, remove_prefix: &str) -> String {
    pascal_case(trim_table_prefix(table_name, remove_prefix))
}

/// 去掉表名前缀（前缀为空或未命中时原样返回）
fn trim_table_prefix<'a>(table_name: &'a str, remove_prefix: &str) -> &'a str {
    if remove_prefix.is_empty() {
        return table_name;
    }
    table_name.strip_prefix(remove_prefix).unwrap_or(table_name)
}

/// 权限标识前缀（如 sys:article）：路由守卫、前端 v-hasPerm 与菜单按钮权限共用同一个值
fn resolve_permission_prefix(module_name: &str, entity_name: &str) -> String {
    let namespace = PERMISSION_NAMESPACES
        .iter()
        .find(|(module, _)| *module == module_name)
        .map_or(module_name, |(_, namespace)| *namespace);
    format!("{namespace}:{}", kebab_case(entity_name))
}

/// 表名 → PascalCase 实体名（sys_user → SysUser）
fn pascal_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// 下划线/短横线命名 → camelCase
fn camel_case(s: &str) -> String {
    let mut out = String::new();
    for (index, part) in s.split(['_', '-']).filter(|p| !p.is_empty()).enumerate() {
        let mut chars = part.chars();
        let Some(first) = chars.next() else {
            continue;
        };
        if index == 0 {
            out.extend(first.to_lowercase());
        } else {
            out.extend(first.to_uppercase());
        }
        out.push_str(chars.as_str());
    }
    if out.is_empty() {
        s.to_lowercase()
    } else {
        out
    }
}

/// 列名 → 合法 Rust 标识符（关键字加 r# 前缀）
fn rust_ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do",
        "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in",
        "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
        "return", "static", "struct", "trait", "true", "try", "type", "typeof", "union",
        "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
    ];

    let candidate = if name.is_empty() { "field" } else { name };
    if matches!(candidate, "self" | "Self" | "super" | "crate") {
        format!("{candidate}_")
    } else if KEYWORDS.contains(&candidate) {
        format!("r#{candidate}")
    } else {
        candidate.to_string()
    }
}

/// PostgreSQL 类型 → Rust 类型
fn pg_to_rust(udt: &str, _data_type: &str) -> &'static str {
    match udt {
        "int2" | "smallint" => "i16",
        "int4" | "integer" => "i32",
        "int8" | "bigint" => "i64",
        "float4" | "real" => "f32",
        "float8" | "double precision" => "f64",
        "numeric" | "decimal" => "f64",
        "bool" | "boolean" => "bool",
        "uuid" => "Uuid",
        "json" | "jsonb" => "JsonValue",
        "timestamp" | "timestamptz" | "date" | "time" => "NaiveDateTime",
        _ => "String",
    }
}

/// Rust 类型 → TypeScript 类型
fn rust_to_ts(rust_type: &str) -> &'static str {
    match rust_type {
        "i16" | "i32" | "i64" | "f32" | "f64" => "number",
        "bool" => "boolean",
        "String" | "Uuid" | "NaiveDateTime" | "JsonValue" => "string",
        _ => "any",
    }
}

fn default_form_type(rust_type: &str, field_name: &str) -> String {
    match field_name {
        n if n.contains("status") => "SWITCH".into(),
        n if n.contains("time") || n.contains("date") => "DATE_TIME".into(),
        n if n.contains("remark") || n.contains("desc") || n.contains("content") => "TEXT_AREA".into(),
        _ => match rust_type {
            "i16" | "i32" | "i64" | "f32" | "f64" => "INPUT_NUMBER".into(),
            "bool" => "SWITCH".into(),
            _ => "INPUT".into(),
        },
    }
}

fn default_query_type(rust_type: &str) -> String {
    match rust_type {
        "String" | "Uuid" => "LIKE".into(),
        _ => "EQ".into(),
    }
}

fn form_type_name_to_int(name: &str) -> i16 {
    match name {
        "INPUT" => 1,
        "SELECT" => 2,
        "RADIO" => 3,
        "CHECK_BOX" => 4,
        "INPUT_NUMBER" => 5,
        "SWITCH" => 6,
        "TEXT_AREA" => 7,
        "DATE" => 8,
        "DATE_TIME" => 9,
        "HIDDEN" => 10,
        _ => 1,
    }
}

fn int_to_form_type_name(v: i16) -> String {
    match v {
        1 => "INPUT",
        2 => "SELECT",
        3 => "RADIO",
        4 => "CHECK_BOX",
        5 => "INPUT_NUMBER",
        6 => "SWITCH",
        7 => "TEXT_AREA",
        8 => "DATE",
        9 => "DATE_TIME",
        10 => "HIDDEN",
        _ => "INPUT",
    }
    .into()
}

fn query_type_name_to_int(name: &str) -> i16 {
    match name {
        "EQ" => 1,
        "LIKE" => 2,
        "IN" => 3,
        "BETWEEN" => 4,
        "GT" => 5,
        "GE" => 6,
        "LT" => 7,
        "LE" => 8,
        "NE" => 9,
        "LIKE_LEFT" => 10,
        "LIKE_RIGHT" => 11,
        _ => 1,
    }
}

fn int_to_query_type_name(v: i16) -> String {
    match v {
        1 => "EQ",
        2 => "LIKE",
        3 => "IN",
        4 => "BETWEEN",
        5 => "GT",
        6 => "GE",
        7 => "LT",
        8 => "LE",
        9 => "NE",
        10 => "LIKE_LEFT",
        11 => "LIKE_RIGHT",
        _ => "EQ",
    }
    .into()
}
