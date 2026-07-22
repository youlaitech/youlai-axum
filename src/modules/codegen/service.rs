//! 代码生成服务：PostgreSQL 元数据读取 + 模板渲染 + zip 打包

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde::Serialize;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::error::{AppError, AppResult};
use crate::modules::codegen::templates;
use crate::result_code::ResultCode;

/// 模板上下文：字段信息
#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub rust_type: String,
    pub is_pk: bool,
    pub is_nullable: bool,
    pub is_string: bool,
    pub is_datetime: bool,
    pub comment: String,
}

/// 模板上下文：整表元数据
#[derive(Debug, Clone, Serialize)]
pub struct TableMeta {
    pub table_name: String,
    pub entity_name: String,
    pub entity_lower: String,
    pub module_name: String,
    pub entity_kebab: String,
    pub pk_name: String,
    pub columns: Vec<ColumnInfo>,
    pub has_datetime: bool,
    pub has_json: bool,
    pub has_uuid: bool,
    pub has_decimal: bool,
}

/// 元数据中读取的原始列信息
struct RawColumn {
    name: String,
    udt: String,
    data_type: String,
    is_nullable: bool,
    comment: String,
    is_pk: bool,
}

/// 代码生成服务
pub struct CodegenService<'a> {
    db: &'a sea_orm::DatabaseConnection,
}

impl<'a> CodegenService<'a> {
    pub fn new(db: &'a sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }

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

    /// 构造模板上下文（含字段类型映射与模块/实体命名推导）
    async fn load_meta(&self, table_name: &str) -> AppResult<TableMeta> {
        let raw = self.get_columns(table_name).await?;
        if raw.is_empty() {
            return Err(AppError::Business(
                ResultCode::UserRequestParameterError,
                Some(format!("表 {table_name} 不存在或无字段")),
            ));
        }

        let entity_name = pascal_case(table_name);
        let entity_lower = lower_first(&entity_name);
        let module_name = table_name
            .split('_')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("app")
            .to_string();
        let entity_kebab = table_name.replace('_', "-");
        let pk_name = raw
            .iter()
            .find(|c| c.is_pk)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "id".to_string());

        let mut has_datetime = false;
        let mut has_json = false;
        let mut has_uuid = false;
        let mut has_decimal = false;

        let columns: Vec<ColumnInfo> = raw
            .iter()
            .map(|c| {
                let rust_type = pg_to_rust(&c.udt, &c.data_type).to_string();
                let is_string = matches!(
                    c.udt.as_str(),
                    "text" | "varchar" | "char" | "bpchar" | "name" | "citext"
                );
                let is_datetime = matches!(
                    c.udt.as_str(),
                    "timestamp" | "timestamptz" | "date" | "time"
                );
                let is_uuid = c.udt == "uuid";
                let is_decimal = matches!(c.udt.as_str(), "numeric" | "decimal");
                has_datetime |= is_datetime;
                has_json |= matches!(c.udt.as_str(), "json" | "jsonb");
                has_uuid |= is_uuid;
                has_decimal |= is_decimal;
                ColumnInfo {
                    name: c.name.clone(),
                    rust_type,
                    is_pk: c.is_pk,
                    is_nullable: c.is_nullable,
                    is_string,
                    is_datetime,
                    comment: c.comment.clone(),
                }
            })
            .collect();

        Ok(TableMeta {
            table_name: table_name.to_string(),
            entity_name,
            entity_lower,
            module_name,
            entity_kebab,
            pk_name,
            columns,
            has_datetime,
            has_json,
            has_uuid,
            has_decimal,
        })
    }

    /// 预览生成的代码（不打包）
    pub async fn preview(&self, table_name: &str) -> AppResult<Vec<templates::GeneratedFile>> {
        let meta = self.load_meta(table_name).await?;
        Ok(templates::render(&meta)?)
    }

    /// 生成代码并打包为 zip 字节流
    pub async fn download(&self, table_name: &str) -> AppResult<Vec<u8>> {
        let meta = self.load_meta(table_name).await?;
        let files = templates::render(&meta)?;

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

    /// 生成代码生成配置（由元数据推导，不持久化）
    pub async fn config(&self, table_name: &str) -> AppResult<TableMeta> {
        self.load_meta(table_name).await
    }
}

fn zip_err(e: zip::result::ZipError) -> AppError {
    AppError::Business(ResultCode::SystemError, Some(e.to_string()))
}

/// 表名 → PascalCase 实体名（sys_user -> SysUser）
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

/// 首字母小写
fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(f) => f.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
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
        "numeric" | "decimal" => "i64",
        "bool" | "boolean" => "bool",
        "uuid" => "Uuid",
        "json" | "jsonb" => "JsonValue",
        "timestamp" | "timestamptz" | "date" | "time" => "NaiveDateTime",
        _ => "String",
    }
}
