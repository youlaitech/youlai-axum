//! 代码生成模板（Tera）与渲染逻辑
//!
//! 从 PostgreSQL 元数据构造模板上下文，渲染出 SeaORM 实体 / DTO / Service / Router
//! 四类 Rust 源码，并打包为文件列表。

use tera::{Context, Tera};

use super::service::TableMeta;

/// 单个待生成文件
pub struct GeneratedFile {
    /// zip 内的相对路径（不含文件名）
    pub path: String,
    /// 文件名
    pub file_name: String,
    /// 文件内容
    pub content: String,
}

/// SeaORM 实体模板
const ENTITY_TMPL: &str = r#"use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
{% if has_datetime %}
use chrono::NaiveDateTime;
{% endif %}{% if has_uuid %}
use uuid::Uuid;
{% endif %}{% if has_json %}
use serde_json::Value as JsonValue;
{% endif %}

/// 由代码生成器基于表 `{{ table_name }}` 生成，请勿手动修改
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "{{ table_name }}")]
pub struct Model {
{% for c in columns %}{% if c.is_pk %}    #[sea_orm(primary_key)]
{% endif %}    pub {{ c.name }}: {% if c.is_nullable %}Option<{% endif %}{{ c.rust_type }}{% if c.is_nullable %}>{% endif %},
{% endfor %}

}

/// 实体别名（便于在 Service / Handler 中以实体名引用）
pub type {{ entity_name }} = Model;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
"#;

/// DTO 模板（查询条件 + 创建/更新表单）
const DTO_TMPL: &str = r#"use serde::{Deserialize, Serialize};
use validator::Validate;
{% if has_datetime %}
use chrono::NaiveDateTime;
{% endif %}

/// 由代码生成器基于表 `{{ table_name }}` 生成的查询条件
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct {{ entity_name }}Query {
    pub keywords: Option<String>,
{% for c in columns %}{% if not c.is_pk and c.is_string %}    pub {{ c.name }}: Option<String>,
{% endif %}{% endfor %}

}

/// 由代码生成器基于表 `{{ table_name }}` 生成的创建/更新表单
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct {{ entity_name }}Form {
{% for c in columns %}{% if not c.is_pk %}{% if c.is_string %}    #[validate(length(min = 1))]
{% endif %}    pub {{ c.name }}: {% if c.is_nullable %}Option<{% endif %}{{ c.rust_type }}{% if c.is_nullable %}>{% endif %},
{% endif %}{% endfor %}

}
"#;

/// Service 模板
const SERVICE_TMPL: &str = r#"use sea_orm::*;
use crate::error::{AppError, AppResult};
use crate::result_code::ResultCode;
use super::entity::*;
use super::dto::*;

/// 由代码生成器基于表 `{{ table_name }}` 生成的服务
pub struct {{ entity_name }}Service<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> {{ entity_name }}Service<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list(&self) -> AppResult<Vec<{{ entity_name }}>> {
        let list = Entity::find().all(self.db).await?;
        Ok(list)
    }

    pub async fn create(&self, form: &{{ entity_name }}Form) -> AppResult<i64> {
        let active = form_to_active(form);
        let model = active.insert(self.db).await?;
        Ok(model.{{ pk_name }})
    }

    pub async fn get(&self, id: i64) -> AppResult<Option<{{ entity_name }}>> {
        Ok(Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn update(&self, id: i64, form: &{{ entity_name }}Form) -> AppResult<()> {
        let mut active: ActiveModel = Entity::find_by_id(id)
            .one(self.db).await?
            .ok_or_else(|| AppError::Business(ResultCode::AccountNotFound, None))?
            .into();
{% for c in columns %}{% if not c.is_pk %}        active.{{ c.name }} = Set(form.{{ c.name }}.clone());
{% endif %}{% endfor %}        active.update(self.db).await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> AppResult<u64> {
        Ok(Entity::delete_by_id(id).exec(self.db).await?.rows_affected)
    }
}

fn form_to_active(form: &{{ entity_name }}Form) -> ActiveModel {
    ActiveModel {
{% for c in columns %}{% if not c.is_pk %}        {{ c.name }}: Set(form.{{ c.name }}.clone()),
{% endif %}{% endfor %}        ..Default::default()
    }
}
"#;

/// Router + Handler 模板
const MOD_TMPL: &str = r#"pub mod entity;
pub mod dto;
pub mod service;

use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use crate::state::AppState;
use crate::error::{AppError, AppResult};
use crate::response::Result;
use crate::result_code::ResultCode;
use dto::*;
use entity::SysUser;
use service::{{ entity_name }}Service;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/{{ entity_kebab }}", get(list).post(create))
        .route("/api/v1/{{ entity_kebab }}/{id}", get(get_one).put(update).delete(delete_one))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Result<Vec<{{ entity_name }}>>>> {
    let data = {{ entity_name }}Service::new(&state.db).list().await?;
    Ok(Json(Result::success(data)))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Result<{{ entity_name }}>>> {
    let data = {{ entity_name }}Service::new(&state.db).get(id).await?
        .ok_or_else(|| AppError::Business(ResultCode::AccountNotFound, None))?;
    Ok(Json(Result::success(data)))
}

async fn create(State(state): State<AppState>, Json(form): Json<{{ entity_name }}Form>) -> AppResult<Json<Result<i64>>> {
    let id = {{ entity_name }}Service::new(&state.db).create(&form).await?;
    Ok(Json(Result::success(id)))
}

async fn update(State(state): State<AppState>, Path(id): Path<i64>, Json(form): Json<{{ entity_name }}Form>) -> AppResult<Json<Result<()>>> {
    {{ entity_name }}Service::new(&state.db).update(id, &form).await?;
    Ok(Json(Result::success_empty()))
}

async fn delete_one(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Result<u64>>> {
    let n = {{ entity_name }}Service::new(&state.db).delete(id).await?;
    Ok(Json(Result::success(n)))
}
"#;

/// 渲染全部模板，返回文件列表
pub fn render(meta: &TableMeta) -> anyhow::Result<Vec<GeneratedFile>> {
    let mut tera = Tera::default();
    tera.add_raw_template("entity", ENTITY_TMPL)?;
    tera.add_raw_template("dto", DTO_TMPL)?;
    tera.add_raw_template("service", SERVICE_TMPL)?;
    tera.add_raw_template("mod", MOD_TMPL)?;

    let ctx = Context::from_serialize(meta)?;

    let files = [
        (
            "entity",
            format!("src/modules/{}/{}", meta.module_name, meta.entity_lower),
            "entity.rs".to_string(),
        ),
        (
            "dto",
            format!("src/modules/{}/{}", meta.module_name, meta.entity_lower),
            "dto.rs".to_string(),
        ),
        (
            "service",
            format!("src/modules/{}/{}", meta.module_name, meta.entity_lower),
            "service.rs".to_string(),
        ),
        (
            "mod",
            format!("src/modules/{}/{}", meta.module_name, meta.entity_lower),
            "mod.rs".to_string(),
        ),
    ];

    let mut out = Vec::with_capacity(files.len());
    for (tpl, path, file_name) in files {
        let content = tera.render(tpl, &ctx)?;
        out.push(GeneratedFile {
            path,
            file_name,
            content,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::codegen::service::{ColumnInfo, TableMeta};

    fn sample_meta() -> TableMeta {
        TableMeta {
            table_name: "sys_user".into(),
            entity_name: "SysUser".into(),
            entity_lower: "sysUser".into(),
            module_name: "sys".into(),
            entity_kebab: "sys-user".into(),
            pk_name: "id".into(),
            columns: vec![
                ColumnInfo {
                    name: "id".into(),
                    rust_type: "i64".into(),
                    is_pk: true,
                    is_nullable: false,
                    is_string: false,
                    is_datetime: false,
                    comment: "主键".into(),
                },
                ColumnInfo {
                    name: "username".into(),
                    rust_type: "String".into(),
                    is_pk: false,
                    is_nullable: false,
                    is_string: true,
                    is_datetime: false,
                    comment: "用户名".into(),
                },
                ColumnInfo {
                    name: "create_time".into(),
                    rust_type: "NaiveDateTime".into(),
                    is_pk: false,
                    is_nullable: true,
                    is_string: false,
                    is_datetime: true,
                    comment: "创建时间".into(),
                },
            ],
            has_datetime: true,
            has_json: false,
            has_uuid: false,
            has_decimal: false,
        }
    }

    #[test]
    fn render_produces_four_compilable_files() {
        let files = render(&sample_meta()).expect("render failed");
        assert_eq!(files.len(), 4);

        let entity = files
            .iter()
            .find(|f| f.file_name == "entity.rs")
            .expect("entity file");
        assert!(entity.content.contains("pub struct Model"));
        assert!(entity.content.contains("pub type SysUser = Model;"));
        assert!(entity
            .content
            .contains("impl ActiveModelBehavior for ActiveModel"));

        let service = files
            .iter()
            .find(|f| f.file_name == "service.rs")
            .expect("service file");
        assert!(service.content.contains("Entity::find()"));
        assert!(service.content.contains("Entity::find_by_id(id)"));
        assert!(service.content.contains("Ok(model.id)"));
        assert!(service.content.contains("form_to_active"));
        assert!(!service.content.contains("SysUser::Entity"));

        let dto = files
            .iter()
            .find(|f| f.file_name == "dto.rs")
            .expect("dto file");
        assert!(dto.content.contains("struct SysUserForm"));
        // 仅字符串字段（username）应有 length 校验，create_time 不应有
        assert_eq!(
            dto.content.matches("#[validate(length(min = 1))]").count(),
            1
        );
        assert!(dto.content.contains("pub username") && dto.content.contains("pub create_time"));

        let m = files
            .iter()
            .find(|f| f.file_name == "mod.rs")
            .expect("mod file");
        assert!(m.content.contains("Router<AppState>"));
        assert!(m.content.contains("SysUserService"));
    }
}
