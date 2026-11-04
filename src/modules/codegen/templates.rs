//! 代码生成模板（Tera）与渲染逻辑

use super::service::TableMeta;
use tera::{Context, Tera};

pub struct GeneratedFile {
    pub path: String,
    pub file_name: String,
    pub content: String,
    pub scope: String,
}

// 后端模板 — 生成 SeaORM 实体 / DTO / Service / Router 四类 Rust 源码

const ENTITY_TMPL: &str = r#"use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
{% if has_datetime %}
use chrono::NaiveDateTime;
{% endif %}{% if has_uuid %}
use uuid::Uuid;
{% endif %}{% if has_json %}
use serde_json::Value as JsonValue;
{% endif %}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "{{ table_name }}")]
pub struct Model {
{% for c in columns %}{% if c.is_pk %}    #[sea_orm(primary_key)]
{% endif %}    pub {{ c.rust_field_name }}: {% if c.is_nullable %}Option<{% endif %}{{ c.rust_type }}{% if c.is_nullable %}>{% endif %},
{% endfor -%}
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
"#;

const DTO_TMPL: &str = r#"use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct {{ entity_name }}Query {
    pub page_num: Option<u64>,
    pub page_size: Option<u64>,
{% for c in columns %}{% if c.is_show_in_query %}    #[serde(default, deserialize_with = "crate::util::opt_string_from_any")]
    pub {{ c.rust_field_name }}: Option<String>,
{% endif %}{% endfor -%}
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct {{ entity_name }}Form {
{% for c in columns %}{% if c.is_editable %}{% if c.is_string and c.is_required %}    #[validate(length(min = 1))]
{% endif %}    pub {{ c.rust_field_name }}: {% if c.is_nullable %}Option<{% endif %}{{ c.rust_type }}{% if c.is_nullable %}>{% endif %},
{% endif %}{% endfor -%}
}
"#;

const SERVICE_TMPL: &str = r#"use sea_orm::*;

use crate::error::{AppError, AppResult};
use crate::result_code::ResultCode;

use super::dto::*;
use crate::entity::{{ table_name }}::*;

pub struct {{ entity_name }}Service<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> {{ entity_name }}Service<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    pub async fn list(&self, query: &{{ entity_name }}Query) -> AppResult<(Vec<Model>, u64)> {
        let mut select = Entity::find(){% if has_soft_delete %}.filter(Column::IsDeleted.eq(0)){% endif %};
{% for c in columns %}{% if c.is_show_in_query %}{% if c.is_string %}        if let Some(ref v) = query.{{ c.rust_field_name }} { select = select.filter(Column::{{ c.column_pascal }}.contains(v)); }
{% elif c.is_integer %}        if let Some(ref v) = query.{{ c.rust_field_name }} { if let Ok(val) = v.parse::<{{ c.rust_type }}>() { select = select.filter(Column::{{ c.column_pascal }}.eq(val)); } }
{% elif c.is_bool %}        if let Some(ref v) = query.{{ c.rust_field_name }} { let val = v == "1" || v.eq_ignore_ascii_case("true"); select = select.filter(Column::{{ c.column_pascal }}.eq(val)); }
{% endif %}{% endif %}{% endfor %}        let total = select.clone().count(self.db).await?;
        let page = query.page_num.unwrap_or(1).max(1);
        let size = query.page_size.unwrap_or(10).clamp(1, 1000);
        let list = select.order_by_desc(Column::{{ pk_column_pascal }}).offset((page - 1) * size).limit(size).all(self.db).await?;
        Ok((list, total))
    }

    pub async fn create(&self, form: &{{ entity_name }}Form, user_id: i64) -> AppResult<i64> {
        let mut active = form_to_active(form);
{% if has_create_by %}        active.create_by = Set({% if create_by_nullable %}Some(user_id){% else %}user_id{% endif %});
{% endif %}{% if has_update_by %}        active.update_by = Set({% if update_by_nullable %}Some(user_id){% else %}user_id{% endif %});
{% endif %}{% if has_create_time %}        active.create_time = Set({% if create_time_nullable %}Some(chrono::Utc::now().naive_utc()){% else %}chrono::Utc::now().naive_utc(){% endif %});
{% endif %}{% if has_update_time %}        active.update_time = Set({% if update_time_nullable %}Some(chrono::Utc::now().naive_utc()){% else %}chrono::Utc::now().naive_utc(){% endif %});
{% endif %}{% if has_soft_delete %}        active.is_deleted = Set(0);
{% endif %}        let model = active.insert(self.db).await?;
        Ok(model.{{ pk_field_name }})
    }

    pub async fn get(&self, id: i64) -> AppResult<Option<Model>> {
        Ok(Entity::find_by_id(id){% if has_soft_delete %}.filter(Column::IsDeleted.eq(0)){% endif %}.one(self.db).await?)
    }

    pub async fn update(&self, id: i64, form: &{{ entity_name }}Form, user_id: i64) -> AppResult<()> {
        let mut active: ActiveModel = Entity::find_by_id(id){% if has_soft_delete %}.filter(Column::IsDeleted.eq(0)){% endif %}
            .one(self.db).await?
            .ok_or_else(|| AppError::Business(ResultCode::DataNotFound, None))?
            .into();
{% for c in columns %}{% if c.is_editable %}        active.{{ c.rust_field_name }} = Set(form.{{ c.rust_field_name }}.clone());
{% endif %}{% endfor %}{% if has_update_by %}        active.update_by = Set({% if update_by_nullable %}Some(user_id){% else %}user_id{% endif %});
{% endif %}{% if has_update_time %}        active.update_time = Set({% if update_time_nullable %}Some(chrono::Utc::now().naive_utc()){% else %}chrono::Utc::now().naive_utc(){% endif %});
{% endif %}        active.update(self.db).await?;
        Ok(())
    }

    pub async fn delete(&self, ids: &str) -> AppResult<u64> {
        let id_list: Vec<i64> = ids.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if id_list.is_empty() {
            return Err(AppError::Business(ResultCode::RequestRequiredParameterIsEmpty, None));
        }
{% if has_soft_delete %}        Ok(sea_orm::Update::many(Entity)
            .col_expr(Column::IsDeleted, sea_orm::sea_query::Expr::value(1)){% if has_update_time %}
            .col_expr(Column::UpdateTime, sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc())){% endif %}
            .filter(Column::{{ pk_column_pascal }}.is_in(id_list))
            .exec(self.db).await?.rows_affected)
{% else %}        Ok(Entity::delete_many().filter(Column::{{ pk_column_pascal }}.is_in(id_list)).exec(self.db).await?.rows_affected)
{% endif %}    }
}

fn form_to_active(form: &{{ entity_name }}Form) -> ActiveModel {
    ActiveModel {
{% for c in columns %}{% if c.is_editable %}        {{ c.rust_field_name }}: Set(form.{{ c.rust_field_name }}.clone()),
{% endif %}{% endfor %}        ..Default::default()
    }
}
"#;

const MOD_TMPL: &str = r#"pub mod dto;
pub mod service;

use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{get, put},
    Json, Router,
};
use validator::Validate;

use crate::error::{AppError, AppResult};
use crate::modules::auth::extractor::AuthUser;
use crate::modules::auth::permission::require_perm;
use crate::response::{PageResult, Result};
use crate::result_code::ResultCode;
use crate::state::AppState;

use crate::entity::{{ table_name }}::Model;
use self::dto::*;
use self::service::{{ entity_name }}Service;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/{{ entity_kebab_plural }}",
            get(list)
                .route_layer(middleware::from_fn(require_perm("{{ permission_prefix }}:list")))
                .post(create)
                .route_layer(middleware::from_fn(require_perm("{{ permission_prefix }}:create"))),
        )
        .route(
            "/api/v1/{{ entity_kebab_plural }}/{id}/form",
            get(get_one).route_layer(middleware::from_fn(require_perm("{{ permission_prefix }}:update"))),
        )
        .route(
            "/api/v1/{{ entity_kebab_plural }}/{id}",
            put(update)
                .route_layer(middleware::from_fn(require_perm("{{ permission_prefix }}:update")))
                .delete(delete_one)
                .route_layer(middleware::from_fn(require_perm("{{ permission_prefix }}:delete"))),
        )
}

async fn list(State(state): State<AppState>, Query(q): Query<{{ entity_name }}Query>) -> AppResult<Json<PageResult<Model>>> {
    let (data, total) = {{ entity_name }}Service::new(&state.db).list(&q).await?;
    Ok(Json(PageResult::success(data, total)))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Result<Model>>> {
    let data = {{ entity_name }}Service::new(&state.db).get(id).await?
        .ok_or_else(|| AppError::Business(ResultCode::DataNotFound, None))?;
    Ok(Json(Result::success(data)))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(form): Json<{{ entity_name }}Form>,
) -> AppResult<Json<Result<i64>>> {
    form.validate()?;
    let id = {{ entity_name }}Service::new(&state.db).create(&form, user.user_id).await?;
    Ok(Json(Result::success(id)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AuthUser(user): AuthUser,
    Json(form): Json<{{ entity_name }}Form>,
) -> AppResult<Json<Result<()>>> {
    form.validate()?;
    {{ entity_name }}Service::new(&state.db).update(id, &form, user.user_id).await?;
    Ok(Json(Result::success_empty()))
}

async fn delete_one(State(state): State<AppState>, Path(ids): Path<String>) -> AppResult<Json<Result<u64>>> {
    let count = {{ entity_name }}Service::new(&state.db).delete(&ids).await?;
    Ok(Json(Result::success(count)))
}
"#;

// 前端模板 — 直接从 boot .vm 搬运，仅替换模板语法
// 使用 [[ ]] 输出分隔符，避免与 Vue {{ }} 冲突

// ── TS: API 调用层 ──

const TS_API_TMPL: &str = r##"import request from "@/utils/request";
import type { [[ entity_name ]]Form, [[ entity_name ]]QueryParams, [[ entity_name ]]Item } from "./types";
import type { PageResult } from "@/api/common";

const BASE_URL = "/api/v1/[[ entity_kebab_plural ]]";

const [[ entity_name ]]API = {
    /** 获取[[ business_name ]]分页数据 */
    getPage(queryParams?: [[ entity_name ]]QueryParams) {
        return request<unknown, PageResult<[[ entity_name ]]Item>>({
            url: `${BASE_URL}`,
            method: "get",
            params: queryParams,
        });
    },
    /** 获取[[ business_name ]]表单数据 */
    getFormData(id: string) {
        return request<unknown, [[ entity_name ]]Form>({
            url: `${BASE_URL}/${id}/form`,
            method: "get",
        });
    },
    /** 新增[[ business_name ]] */
    create(data: [[ entity_name ]]Form) {
        return request({
            url: `${BASE_URL}`,
            method: "post",
            data,
        });
    },
    /** 更新[[ business_name ]] */
    update(id: string, data: [[ entity_name ]]Form) {
        return request({
            url: `${BASE_URL}/${id}`,
            method: "put",
            data,
        });
    },
    /** 批量删除[[ business_name ]]，多个以英文逗号(,)分割 */
    deleteByIds(ids: string) {
        return request({
            url: `${BASE_URL}/${ids}`,
            method: "delete",
        });
    }
}

export default [[ entity_name ]]API;

export * from "./types";
"##;

// ── TS: 类型定义 ──

const TS_TYPES_TMPL: &str = r##"/**
 * [[ entity_name ]] [[ business_name ]]类型定义
 */

import type { PageQuery } from "@/api/common";

/** [[ business_name ]]查询参数 */
export interface [[ entity_name ]]QueryParams extends PageQuery {
{% for c in columns %}{% if c.is_show_in_query %}{% if c.field_comment != "" %}  /** [[ c.field_comment ]] */
{% endif %}{% if c.form_type_name == "DATE" or c.form_type_name == "DATE_TIME" %}{% if c.query_type_name == "BETWEEN" %}  [[ c.field_name ]]?: [string, string];
{% else %}  [[ c.field_name ]]?: [[ c.ts_type ]];
{% endif %}{% else %}  [[ c.field_name ]]?: [[ c.ts_type ]];
{% endif %}{% endif %}{% endfor -%}
}

/** [[ business_name ]]表单对象 */
export interface [[ entity_name ]]Form {
  id?: string;
{% for c in columns %}{% if c.is_show_in_form %}{% if c.field_comment != "" %}  /** [[ c.field_comment ]] */
{% endif %}  [[ c.field_name ]]?: [[ c.ts_type ]];
{% endif %}{% endfor -%}
}

/** [[ business_name ]]详情对象 */
export type [[ entity_name ]]Detail = [[ entity_name ]]Item & [[ entity_name ]]Form;

/** [[ business_name ]]列表项 */
export interface [[ entity_name ]]Item {
{% for c in columns %}{% if c.is_show_in_list %}{% if c.field_comment != "" %}  /** [[ c.field_comment ]] */
{% endif %}  [[ c.field_name ]]?: [[ c.ts_type ]];
{% endif %}{% endfor -%}
}
"##;

// ── TS: classic 页面 ──

const TS_CLASSIC_VUE_TMPL: &str = r##"<template>
  <div class="page-container">
    <el-card class="page-search" shadow="never">
      <el-form ref="queryFormRef" :model="params" :inline="true">
{% for c in columns %}{% if c.is_show_in_query %}        <el-form-item label="[[ c.field_comment ]]" prop="[[ c.field_name ]]">
{% if c.form_type_name == "INPUT" %}          <el-input v-model="params.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" clearable @keyup.enter="handleQuery()" />
{% elif c.form_type_name == "SELECT" %}{% if c.dict_type != "" %}          <DictSelect v-model="params.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
{% else %}          <el-select v-model="params.[[ c.field_name ]]" placeholder="请选择[[ c.field_comment ]]">
            <el-option :key="1" :value="1" label="选项一"/>
            <el-option :key="2" :value="2" label="选项二"/>
          </el-select>
{% endif %}{% elif c.form_type_name == "RADIO" %}{% if c.dict_type != "" %}          <DictSelect v-model="params.[[ c.field_name ]]" type="radio" code="[[ c.dict_type ]]" />
{% else %}          <el-radio-group v-model="params.[[ c.field_name ]]">
            <el-radio :key="1" :value="1">选项一</el-radio>
            <el-radio :key="2" :value="2">选项二</el-radio>
          </el-radio-group>
{% endif %}{% elif c.form_type_name == "CHECK_BOX" %}{% if c.dict_type != "" %}          <DictSelect v-model="params.[[ c.field_name ]]" type="checkbox" code="[[ c.dict_type ]]" />
{% else %}          <el-checkbox-group v-model="params.[[ c.field_name ]]">
            <el-checkbox :key="1" :label="1">选项一</el-checkbox>
            <el-checkbox :key="2" :label="2">选项二</el-checkbox>
          </el-checkbox-group>
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}          <el-input-number v-model="params.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "SWITCH" %}          <el-switch v-model="params.[[ c.field_name ]]" :active-value="1" :inactive-value="0" />
{% elif c.form_type_name == "TEXT_AREA" %}          <el-input type="textarea" v-model="params.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "DATE_TIME" %}{% if c.query_type_name == "BETWEEN" %}          <el-date-picker v-model="params.[[ c.field_name ]]" type="daterange" range-separator="~" start-placeholder="开始时间" end-placeholder="结束时间" value-format="YYYY-MM-DD HH:mm:ss" />
{% else %}          <el-date-picker v-model="params.[[ c.field_name ]]" type="datetime" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD HH:mm:ss" />
{% endif %}{% elif c.form_type_name == "DATE" %}{% if c.query_type_name == "BETWEEN" %}          <el-date-picker class="!w-[240px]" v-model="params.[[ c.field_name ]]" type="daterange" range-separator="~" start-placeholder="开始时间" end-placeholder="结束时间" value-format="YYYY-MM-DD" />
{% else %}          <el-date-picker class="!w-[240px]" v-model="params.[[ c.field_name ]]" type="date" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD" />
{% endif %}{% endif %}        </el-form-item>
{% endif %}{% endfor %}        <el-form-item>
          <el-button type="primary" @click="handleQuery">搜索</el-button>
          <el-button @click="handleResetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card ref="tableWrapperRef" class="page-content" shadow="never">
      <div class="page-toolbar">
        <div class="page-toolbar__left">
          <el-button v-hasPerm="['[[ permission_prefix ]]:create']" type="primary" @click="handleCreateClick()">新增</el-button>
          <el-button v-hasPerm="['[[ permission_prefix ]]:delete']" type="danger" :disabled="!hasSelection" @click="handleBatchDelete()">删除</el-button>
        </div>
        <div class="page-toolbar__right">
          <el-tooltip content="刷新" placement="top">
            <el-button class="page-icon-btn" @click="fetchData"><el-icon><Refresh /></el-icon></el-button>
          </el-tooltip>
          <el-tooltip content="全屏" placement="top">
            <el-button class="page-icon-btn" @click="toggleFullscreen"><el-icon><FullScreen /></el-icon></el-button>
          </el-tooltip>
        </div>
      </div>

      <div class="page-table-wrapper">
        <el-table ref="dataTableRef" v-loading="loading" class="page-table" :data="list" height="100%" border highlight-current-row @selection-change="handleSelectionChange">
        <el-table-column type="selection" width="55" align="center" />
{% for c in columns %}{% if c.is_show_in_list %}{% if c.dict_type != "" %}        <el-table-column label="[[ c.field_comment ]]" width="150" align="center">
          <template #default="scope">
            <DictTag v-model="scope.row.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
          </template>
        </el-table-column>
{% else %}        <el-table-column key="[[ c.field_name ]]" label="[[ c.field_comment ]]" prop="[[ c.field_name ]]" min-width="150" align="center" />
{% endif %}{% endif %}{% endfor %}        <el-table-column fixed="right" label="操作" width="180">
          <template #default="scope">
            <el-button v-hasPerm="['[[ permission_prefix ]]:update']" type="primary" size="small" link @click="handleEditClick(String(scope.row.id))">编辑</el-button>
            <el-button v-hasPerm="['[[ permission_prefix ]]:delete']" type="danger" size="small" link @click="handleDelete(String(scope.row.id))">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      </div>

      <pagination v-if="total > 0" v-model:total="total" v-model:page="params.pageNum" v-model:limit="params.pageSize" class="page-pagination" @pagination="fetchData" />
    </el-card>

    <el-dialog v-model="dialog.visible" :title="dialog.title" width="600px" @close="closeDialog">
      <el-form ref="dataFormRef" :model="formData" :rules="rules" label-width="100px">
{% for c in columns %}{% if c.is_show_in_form and c.form_type_name != "HIDDEN" %}        <el-form-item label="[[ c.field_comment ]]" prop="[[ c.field_name ]]">
{% if c.form_type_name == "INPUT" %}          <el-input v-model="formData.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "SELECT" %}{% if c.dict_type != "" %}          <DictSelect v-model="formData.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
{% else %}          <el-select v-model="formData.[[ c.field_name ]]" placeholder="请选择[[ c.field_comment ]]">
            <el-option :value="0" label="选项一"/>
            <el-option :value="1" label="选项二"/>
          </el-select>
{% endif %}{% elif c.form_type_name == "RADIO" %}{% if c.dict_type != "" %}          <DictSelect v-model="formData.[[ c.field_name ]]" type="radio" code="[[ c.dict_type ]]" />
{% else %}          <el-radio-group v-model="formData.[[ c.field_name ]]">
            <el-radio :value="0">选项一</el-radio>
            <el-radio :value="1">选项二</el-radio>
          </el-radio-group>
{% endif %}{% elif c.form_type_name == "CHECK_BOX" %}{% if c.dict_type != "" %}          <DictSelect v-model="formData.[[ c.field_name ]]" type="checkbox" code="[[ c.dict_type ]]" />
{% else %}          <el-checkbox-group v-model="formData.[[ c.field_name ]]">
            <el-checkbox :value="0">选项一</el-checkbox>
            <el-checkbox :value="1">选项二</el-checkbox>
          </el-checkbox-group>
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}          <el-input-number v-model="formData.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "SWITCH" %}          <el-switch v-model="formData.[[ c.field_name ]]" :active-value="1" :inactive-value="0" />
{% elif c.form_type_name == "TEXT_AREA" %}          <el-input type="textarea" v-model="formData.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "DATE_TIME" %}          <el-date-picker v-model="formData.[[ c.field_name ]]" type="datetime" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD HH:mm:ss" />
{% elif c.form_type_name == "DATE" %}          <el-date-picker class="!w-[240px]" v-model="formData.[[ c.field_name ]]" type="date" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD" />
{% endif %}        </el-form-item>
{% elif c.form_type_name == "HIDDEN" %}        <el-input type="hidden" v-model="formData.[[ c.field_name ]]" />
{% endif %}{% endfor %}      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button type="primary" @click="handleSubmit">确定</el-button>
          <el-button @click="closeDialog">取消</el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { useFullscreen } from "@vueuse/core";
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from "element-plus";
import { FullScreen, Refresh } from "@element-plus/icons-vue";
import { usePageTable, useTableSelection } from "@/composables";
import [[ entity_name ]]API from "@/api/[[ module_name ]]/[[ entity_kebab ]]";
import type { [[ entity_name ]]Item, [[ entity_name ]]Form, [[ entity_name ]]QueryParams } from "@/api/[[ module_name ]]/[[ entity_kebab ]]";

defineOptions({
  name: "[[ entity_name ]]",
  inheritAttrs: false,
});

const queryFormRef = ref<FormInstance>();
const dataFormRef = ref<FormInstance>();
const tableWrapperRef = ref<HTMLElement | null>(null);
const { toggle: toggleFullscreen } = useFullscreen(tableWrapperRef);

const initialFormData = reactive({} as [[ entity_name ]]Form);
Object.assign(initialFormData, {} as [[ entity_name ]]Form);

const { loading, list, total, params, fetchData, handleQuery, handleResetQuery } = usePageTable<
  [[ entity_name ]]Item,
  [[ entity_name ]]QueryParams
>({
  initialParams: {
    pageNum: 1,
    pageSize: 10,
  } as [[ entity_name ]]QueryParams,
  request: [[ entity_name ]]API.getPage,
  onBeforeReset: () => queryFormRef.value?.resetFields(),
});

const { selectedIds, hasSelection, handleSelectionChange } = useTableSelection<[[ entity_name ]]Item>();

const dialog = reactive({
  title: "",
  visible: false,
});

const formData = reactive<[[ entity_name ]]Form>({} as [[ entity_name ]]Form);

const rules: FormRules = {
{% for c in columns %}{% if c.is_show_in_form and c.is_required %}  [[ c.field_name ]]: [{ required: true, message: "请输入[[ c.field_comment ]]", trigger: "blur" }],
{% endif %}{% endfor %}
};

function openDialog(): void {
  dialog.visible = true;
}

function closeDialog(): void {
  dialog.visible = false;
  resetForm();
}

function resetForm(): void {
  dataFormRef.value?.resetFields();
  dataFormRef.value?.clearValidate();
  Object.keys(formData).forEach((key) => {
    delete (formData as Record<string, unknown>)[key];
  });
  Object.assign(formData, initialFormData);
}

async function handleCreateClick(): Promise<void> {
  dialog.title = "新增[[ business_name ]]";
  openDialog();
}

async function handleEditClick(id: string): Promise<void> {
  dialog.title = "修改[[ business_name ]]";
  const data = await [[ entity_name ]]API.getFormData(id);
  Object.assign(formData, data);
  openDialog();
}

async function handleSubmit(): Promise<void> {
  const valid = await dataFormRef.value?.validate().then(
    () => true,
    () => false
  );
  if (!valid) return;

  loading.value = true;
  try {
    const id = formData.id;
    if (id) {
      await [[ entity_name ]]API.update(id, formData);
      ElMessage.success("修改成功");
    } else {
      await [[ entity_name ]]API.create(formData);
      ElMessage.success("新增成功");
    }
    closeDialog();
    handleResetQuery();
  } finally {
    loading.value = false;
  }
}

function handleBatchDelete(): void {
  handleDelete();
}

async function handleDelete(id?: string): Promise<void> {
  const ids = id || selectedIds.value.join(",");
  if (!ids) {
    ElMessage.warning("请勾选删除项");
    return;
  }

  try {
    await ElMessageBox.confirm("确认删除已选中的数据项?", "警告", {
      confirmButtonText: "确定",
      cancelButtonText: "取消",
      type: "warning",
    });
  } catch {
    ElMessage.info("已取消删除");
    return;
  }

  loading.value = true;
  try {
    await [[ entity_name ]]API.deleteByIds(ids);
    ElMessage.success("删除成功");
    handleResetQuery();
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  handleQuery();
});
</script>
"##;

// ── JS: API 调用层 ──

const JS_API_TMPL: &str = r##"import request from "@/utils/request";

const BASE_URL = "/api/v1/[[ entity_kebab_plural ]]";

const [[ entity_name ]]API = {
  /** 获取[[ business_name ]]分页数据 */
  getPage(queryParams) {
    return request({
      url: `${BASE_URL}`,
      method: "get",
      params: queryParams,
    });
  },

  /**
   * 获取[[ business_name ]]表单数据
   * @param {string} id [[ business_name ]]ID
   */
  getFormData(id) {
    return request({
      url: `${BASE_URL}/${id}/form`,
      method: "get",
    });
  },

  /**
   * 新增[[ business_name ]]
   * @param {Object} data [[ business_name ]]表单数据
   */
  create(data) {
    return request({
      url: `${BASE_URL}`,
      method: "post",
      data,
    });
  },

  /**
   * 更新[[ business_name ]]
   * @param {string} id [[ business_name ]]ID
   * @param {Object} data [[ business_name ]]表单数据
   */
  update(id, data) {
    return request({
      url: `${BASE_URL}/${id}`,
      method: "put",
      data,
    });
  },

  /**
   * 批量删除[[ business_name ]]，多个以英文逗号分割
   * @param {string} ids [[ business_name ]]ID字符串
   */
  deleteByIds(ids) {
    return request({
      url: `${BASE_URL}/${ids}`,
      method: "delete",
    });
  },
};

export default [[ entity_name ]]API;
"##;

// ── JS: classic 页面 ──

const JS_CLASSIC_VUE_TMPL: &str = r##"<template>
  <div class="page-container">
    <el-card class="page-search" shadow="never">
      <el-form ref="queryFormRef" :model="params" :inline="true">
{% for c in columns %}{% if c.is_show_in_query %}        <el-form-item label="[[ c.field_comment ]]" prop="[[ c.field_name ]]">
{% if c.form_type_name == "INPUT" %}          <el-input v-model="params.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" clearable @keyup.enter="handleQuery()" />
{% elif c.form_type_name == "SELECT" %}{% if c.dict_type != "" %}          <DictSelect v-model="params.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
{% else %}          <el-select v-model="params.[[ c.field_name ]]" placeholder="请选择[[ c.field_comment ]]">
            <el-option :key="1" :value="1" label="选项一"/>
            <el-option :key="2" :value="2" label="选项二"/>
          </el-select>
{% endif %}{% elif c.form_type_name == "RADIO" %}{% if c.dict_type != "" %}          <DictSelect v-model="params.[[ c.field_name ]]" type="radio" code="[[ c.dict_type ]]" />
{% else %}          <el-radio-group v-model="params.[[ c.field_name ]]">
            <el-radio :key="1" :value="1">选项一</el-radio>
            <el-radio :key="2" :value="2">选项二</el-radio>
          </el-radio-group>
{% endif %}{% elif c.form_type_name == "CHECK_BOX" %}{% if c.dict_type != "" %}          <DictSelect v-model="params.[[ c.field_name ]]" type="checkbox" code="[[ c.dict_type ]]" />
{% else %}          <el-checkbox-group v-model="params.[[ c.field_name ]]">
            <el-checkbox :key="1" :label="1">选项一</el-checkbox>
            <el-checkbox :key="2" :label="2">选项二</el-checkbox>
          </el-checkbox-group>
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}          <el-input-number v-model="params.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "SWITCH" %}          <el-switch v-model="params.[[ c.field_name ]]" :active-value="1" :inactive-value="0" />
{% elif c.form_type_name == "TEXT_AREA" %}          <el-input type="textarea" v-model="params.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "DATE_TIME" %}{% if c.query_type_name == "BETWEEN" %}          <el-date-picker v-model="params.[[ c.field_name ]]" type="daterange" range-separator="~" start-placeholder="开始时间" end-placeholder="结束时间" value-format="YYYY-MM-DD HH:mm:ss" />
{% else %}          <el-date-picker v-model="params.[[ c.field_name ]]" type="datetime" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD HH:mm:ss" />
{% endif %}{% elif c.form_type_name == "DATE" %}{% if c.query_type_name == "BETWEEN" %}          <el-date-picker class="!w-[240px]" v-model="params.[[ c.field_name ]]" type="daterange" range-separator="~" start-placeholder="开始时间" end-placeholder="结束时间" value-format="YYYY-MM-DD" />
{% else %}          <el-date-picker class="!w-[240px]" v-model="params.[[ c.field_name ]]" type="date" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD" />
{% endif %}{% endif %}        </el-form-item>
{% endif %}{% endfor %}        <el-form-item>
          <el-button type="primary" @click="handleQuery">搜索</el-button>
          <el-button @click="handleResetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card ref="tableWrapperRef" class="page-content" shadow="never">
      <div class="page-toolbar">
        <div class="page-toolbar__left">
          <el-button v-hasPerm="['[[ permission_prefix ]]:create']" type="primary" @click="handleCreateClick()">新增</el-button>
          <el-button v-hasPerm="['[[ permission_prefix ]]:delete']" type="danger" :disabled="!hasSelection" @click="handleBatchDelete()">删除</el-button>
        </div>
        <div class="page-toolbar__right">
          <el-tooltip content="刷新" placement="top">
            <el-button class="page-icon-btn" @click="fetchData"><el-icon><Refresh /></el-icon></el-button>
          </el-tooltip>
          <el-tooltip content="全屏" placement="top">
            <el-button class="page-icon-btn" @click="toggleFullscreen"><el-icon><FullScreen /></el-icon></el-button>
          </el-tooltip>
        </div>
      </div>

      <div class="page-table-wrapper">
        <el-table ref="dataTableRef" v-loading="loading" class="page-table" :data="list" height="100%" border highlight-current-row @selection-change="handleSelectionChange">
        <el-table-column type="selection" width="55" align="center" />
{% for c in columns %}{% if c.is_show_in_list %}{% if c.dict_type != "" %}        <el-table-column label="[[ c.field_comment ]]" width="150" align="center">
          <template #default="scope">
            <DictTag v-model="scope.row.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
          </template>
        </el-table-column>
{% else %}        <el-table-column key="[[ c.field_name ]]" label="[[ c.field_comment ]]" prop="[[ c.field_name ]]" min-width="150" align="center" />
{% endif %}{% endif %}{% endfor %}        <el-table-column fixed="right" label="操作" width="180">
          <template #default="scope">
            <el-button v-hasPerm="['[[ permission_prefix ]]:update']" type="primary" size="small" link @click="handleEditClick(String(scope.row.id))">编辑</el-button>
            <el-button v-hasPerm="['[[ permission_prefix ]]:delete']" type="danger" size="small" link @click="handleDelete(String(scope.row.id))">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      </div>

      <pagination v-if="total > 0" v-model:total="total" v-model:page="params.pageNum" v-model:limit="params.pageSize" class="page-pagination" @pagination="fetchData" />
    </el-card>

    <el-dialog v-model="dialog.visible" :title="dialog.title" width="600px" @close="closeDialog">
      <el-form ref="dataFormRef" :model="formData" :rules="rules" label-width="100px">
{% for c in columns %}{% if c.is_show_in_form and c.form_type_name != "HIDDEN" %}        <el-form-item label="[[ c.field_comment ]]" prop="[[ c.field_name ]]">
{% if c.form_type_name == "INPUT" %}          <el-input v-model="formData.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "SELECT" %}{% if c.dict_type != "" %}          <DictSelect v-model="formData.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
{% else %}          <el-select v-model="formData.[[ c.field_name ]]" placeholder="请选择[[ c.field_comment ]]">
            <el-option :value="0" label="选项一"/>
            <el-option :value="1" label="选项二"/>
          </el-select>
{% endif %}{% elif c.form_type_name == "RADIO" %}{% if c.dict_type != "" %}          <DictSelect v-model="formData.[[ c.field_name ]]" type="radio" code="[[ c.dict_type ]]" />
{% else %}          <el-radio-group v-model="formData.[[ c.field_name ]]">
            <el-radio :value="0">选项一</el-radio>
            <el-radio :value="1">选项二</el-radio>
          </el-radio-group>
{% endif %}{% elif c.form_type_name == "CHECK_BOX" %}{% if c.dict_type != "" %}          <DictSelect v-model="formData.[[ c.field_name ]]" type="checkbox" code="[[ c.dict_type ]]" />
{% else %}          <el-checkbox-group v-model="formData.[[ c.field_name ]]">
            <el-checkbox :value="0">选项一</el-checkbox>
            <el-checkbox :value="1">选项二</el-checkbox>
          </el-checkbox-group>
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}          <el-input-number v-model="formData.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "SWITCH" %}          <el-switch v-model="formData.[[ c.field_name ]]" :active-value="1" :inactive-value="0" />
{% elif c.form_type_name == "TEXT_AREA" %}          <el-input type="textarea" v-model="formData.[[ c.field_name ]]" placeholder="[[ c.field_comment ]]" />
{% elif c.form_type_name == "DATE_TIME" %}          <el-date-picker v-model="formData.[[ c.field_name ]]" type="datetime" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD HH:mm:ss" />
{% elif c.form_type_name == "DATE" %}          <el-date-picker class="!w-[240px]" v-model="formData.[[ c.field_name ]]" type="date" placeholder="[[ c.field_comment ]]" value-format="YYYY-MM-DD" />
{% endif %}        </el-form-item>
{% elif c.form_type_name == "HIDDEN" %}        <el-input type="hidden" v-model="formData.[[ c.field_name ]]" />
{% endif %}{% endfor %}      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button type="primary" @click="handleSubmit">确定</el-button>
          <el-button @click="closeDialog">取消</el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { useFullscreen } from "@vueuse/core";
import { ElMessage, ElMessageBox } from "element-plus";
import { FullScreen, Refresh } from "@element-plus/icons-vue";
import { usePageTable, useTableSelection } from "@/composables";
import [[ entity_name ]]API from "@/api/[[ module_name ]]/[[ entity_kebab ]]";

defineOptions({
  name: "[[ entity_name ]]",
  inheritAttrs: false,
});

const queryFormRef = ref();
const dataFormRef = ref();
const tableWrapperRef = ref(null);
const { toggle: toggleFullscreen } = useFullscreen(tableWrapperRef);

const initialFormData = reactive({});

const { loading, list, total, params, fetchData, handleQuery, handleResetQuery } = usePageTable({
  initialParams: {
    pageNum: 1,
    pageSize: 10,
  },
  request: [[ entity_name ]]API.getPage,
  onBeforeReset: () => queryFormRef.value?.resetFields(),
});

const { selectedIds, hasSelection, handleSelectionChange } = useTableSelection();

const dialog = reactive({
  title: "",
  visible: false,
});

const formData = reactive({});

const rules = reactive({
{% for c in columns %}{% if c.is_show_in_form and c.is_required %}  [[ c.field_name ]]: [{ required: true, message: "请输入[[ c.field_comment ]]", trigger: "blur" }],
{% endif %}{% endfor %}
});

function openDialog() {
  dialog.visible = true;
}

function closeDialog() {
  dialog.visible = false;
  resetForm();
}

function resetForm() {
  dataFormRef.value?.resetFields();
  dataFormRef.value?.clearValidate();
  Object.keys(formData).forEach((key) => {
    delete formData[key];
  });
  Object.assign(formData, initialFormData);
}

async function handleCreateClick() {
  dialog.title = "新增[[ business_name ]]";
  openDialog();
}

async function handleEditClick(id) {
  dialog.title = "修改[[ business_name ]]";
  const data = await [[ entity_name ]]API.getFormData(id);
  Object.assign(formData, data);
  openDialog();
}

async function handleSubmit() {
  const valid = await dataFormRef.value?.validate().then(
    () => true,
    () => false
  );
  if (!valid) return;

  loading.value = true;
  try {
    const id = formData.id;
    if (id) {
      await [[ entity_name ]]API.update(id, formData);
      ElMessage.success("修改成功");
    } else {
      await [[ entity_name ]]API.create(formData);
      ElMessage.success("新增成功");
    }
    closeDialog();
    handleResetQuery();
  } finally {
    loading.value = false;
  }
}

function handleBatchDelete() {
  handleDelete();
}

async function handleDelete(id) {
  const ids = id || selectedIds.value.join(",");
  if (!ids) {
    ElMessage.warning("请勾选删除项");
    return;
  }

  try {
    await ElMessageBox.confirm("确认删除已选中的数据项?", "警告", {
      confirmButtonText: "确定",
      cancelButtonText: "取消",
      type: "warning",
    });
  } catch {
    ElMessage.info("已取消删除");
    return;
  }

  loading.value = true;
  try {
    await [[ entity_name ]]API.deleteByIds(ids);
    ElMessage.success("删除成功");
    handleResetQuery();
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  handleQuery();
});
</script>
"##;


const TS_CRUD_VUE_TMPL: &str = r#"<template>
  <CrudPage
    :search-config="searchConfig"
    :table-config="tableConfig"
    :form-config="formConfig"
    :load-form-data="loadFormData"
  >
{% for c in columns %}{% if c.is_show_in_query and c.dict_type %}    <template #search-[[ c.field_name ]]="{ formData, prop, attrs }">
      <DictSelect v-model="formData[prop]" code="[[ c.dict_type ]]" v-bind="attrs" />
    </template>
{% endif %}{% endfor %}{% for c in columns %}{% if c.is_show_in_list and c.dict_type %}    <template #table-cell-[[ c.field_name ]]="{ row }">
      <DictTag :model-value="row.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
    </template>
{% endif %}{% endfor %}{% for c in columns %}{% if c.is_show_in_form and c.form_type_name != "HIDDEN" and c.dict_type %}    <template #form-[[ c.field_name ]]="{ formData, prop, attrs }">
      <DictSelect v-model="formData[prop]" code="[[ c.dict_type ]]" v-bind="attrs" />
    </template>
{% endif %}{% endfor %}  </CrudPage>
</template>

<script setup lang="ts">
import [[ entity_name ]]API from "@/api/[[ module_name ]]/[[ entity_kebab ]]";
import type { [[ entity_name ]]Form, [[ entity_name ]]Item, [[ entity_name ]]QueryParams } from "@/api/[[ module_name ]]/[[ entity_kebab ]]";
import type {
  CrudRecord,
  CrudFormConfig,
  CrudTableConfig,
  CrudSearchConfig,
} from "@/components/Crud/types";

defineOptions({ name: "[[ entity_name ]]", inheritAttrs: false });

const searchConfig: CrudSearchConfig = {
  fields: [
{% for c in columns %}{% if c.is_show_in_query %}    {
      prop: "[[ c.field_name ]]",
      label: "[[ c.field_comment ]]",
{% if c.dict_type %}      type: "custom",
      attrs: { clearable: true },
{% elif c.form_type_name == "DATE" or c.form_type_name == "DATE_TIME" %}      type: "date-picker",
{% if c.query_type_name == "BETWEEN" %}      initialValue: [],
      attrs: {
        type: "daterange",
        valueFormat: "YYYY-MM-DD",
        startPlaceholder: "开始日期",
        endPlaceholder: "结束日期",
      },
{% else %}      attrs: { type: "date", valueFormat: "YYYY-MM-DD", clearable: true },
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}      type: "input-number",
      attrs: { clearable: true },
{% else %}      type: "input",
      attrs: { clearable: true },
{% endif %}    },
{% endif %}{% endfor %}  ],
};

const tableConfig: CrudTableConfig<[[ entity_name ]]QueryParams, [[ entity_name ]]Item> = {
  permissionPrefix: "[[ permission_prefix ]]",
  table: { border: true },
  fetchData: [[ entity_name ]]API.getPage,
  deleteByIds: [[ entity_name ]]API.deleteByIds,
  toolbarActions: ["create", "delete"],
  tableTools: ["refresh", "columns", "search"],
  columns: [
    { type: "selection", width: 50 },
{% for c in columns %}{% if c.is_show_in_list %}    { prop: "[[ c.field_name ]]", label: "[[ c.field_comment ]]", minWidth: 120 },
{% endif %}{% endfor %}    {
      prop: "actions",
      label: "操作",
      cellType: "actions",
      actions: ["view", "edit", "delete"],
      fixed: "right",
      width: 200,
    },
  ],
};

const formConfig: CrudFormConfig<[[ entity_name ]]Form> = {
  title: ({ mode }) => ({
    create: "新增[[ business_name ]]",
    edit: "编辑[[ business_name ]]",
    view: "查看[[ business_name ]]",
  })[mode],
  dialog: { width: 560 },
  /**
   * 保存记录，更新目标使用打开表单时的主键。
   */
  submitForm(data, { mode, id }) {
    if (mode === "create") return [[ entity_name ]]API.create(data);
    if (id === undefined) throw new Error("缺少记录 ID");
    return [[ entity_name ]]API.update(String(id), data);
  },
  fields: [
{% for c in columns %}{% if c.is_show_in_form and c.form_type_name != "HIDDEN" %}    {
      prop: "[[ c.field_name ]]",
      label: "[[ c.field_comment ]]",
{% if c.is_required %}      required: true,
{% endif %}{% if c.dict_type %}      type: "custom",
{% elif c.form_type_name == "SWITCH" %}      type: "switch",
{% if c.ts_type == "boolean" %}      initialValue: true,
      attrs: { activeValue: true, inactiveValue: false },
{% else %}      initialValue: 1,
      attrs: { activeValue: 1, inactiveValue: 0 },
{% endif %}{% elif c.form_type_name == "DATE" or c.form_type_name == "DATE_TIME" %}      type: "date-picker",
{% if c.form_type_name == "DATE_TIME" %}      attrs: { type: "datetime", valueFormat: "YYYY-MM-DD HH:mm:ss" },
{% else %}      attrs: { type: "date", valueFormat: "YYYY-MM-DD" },
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}      type: "input-number",
{% elif c.form_type_name == "SELECT" %}      type: "select",
      options: [],
{% elif c.form_type_name == "RADIO" %}      type: "radio",
      options: [],
{% elif c.form_type_name == "CHECK_BOX" %}      type: "checkbox",
      initialValue: [],
      options: [],
{% else %}      type: "input",
{% if c.form_type_name == "TEXT_AREA" %}      attrs: { type: "textarea", rows: 3 },
{% endif %}{% endif %}    },
{% endif %}{% endfor %}  ],
};

/**
 * 加载编辑和查看所需的详情。
 */
function loadFormData(row: CrudRecord): Promise<[[ entity_name ]]Form> {
  const rowKey = tableConfig.rowKey ?? "id";
  return [[ entity_name ]]API.getFormData(String(row[rowKey]));
}
</script>
"#;

const JS_CRUD_VUE_TMPL: &str = r#"<template>
  <CrudPage
    :search-config="searchConfig"
    :table-config="tableConfig"
    :form-config="formConfig"
    :load-form-data="loadFormData"
  >
{% for c in columns %}{% if c.is_show_in_query and c.dict_type %}    <template #search-[[ c.field_name ]]="{ formData, prop, attrs }">
      <DictSelect v-model="formData[prop]" code="[[ c.dict_type ]]" v-bind="attrs" />
    </template>
{% endif %}{% endfor %}{% for c in columns %}{% if c.is_show_in_list and c.dict_type %}    <template #table-cell-[[ c.field_name ]]="{ row }">
      <DictTag :model-value="row.[[ c.field_name ]]" code="[[ c.dict_type ]]" />
    </template>
{% endif %}{% endfor %}{% for c in columns %}{% if c.is_show_in_form and c.form_type_name != "HIDDEN" and c.dict_type %}    <template #form-[[ c.field_name ]]="{ formData, prop, attrs }">
      <DictSelect v-model="formData[prop]" code="[[ c.dict_type ]]" v-bind="attrs" />
    </template>
{% endif %}{% endfor %}  </CrudPage>
</template>

<script setup>
import [[ entity_name ]]API from "@/api/[[ module_name ]]/[[ entity_kebab ]]";

defineOptions({ name: "[[ entity_name ]]", inheritAttrs: false });

const searchConfig = {
  fields: [
{% for c in columns %}{% if c.is_show_in_query %}    {
      prop: "[[ c.field_name ]]",
      label: "[[ c.field_comment ]]",
{% if c.dict_type %}      type: "custom",
      attrs: { clearable: true },
{% elif c.form_type_name == "DATE" or c.form_type_name == "DATE_TIME" %}      type: "date-picker",
{% if c.query_type_name == "BETWEEN" %}      initialValue: [],
      attrs: {
        type: "daterange",
        valueFormat: "YYYY-MM-DD",
        startPlaceholder: "开始日期",
        endPlaceholder: "结束日期",
      },
{% else %}      attrs: { type: "date", valueFormat: "YYYY-MM-DD", clearable: true },
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}      type: "input-number",
      attrs: { clearable: true },
{% else %}      type: "input",
      attrs: { clearable: true },
{% endif %}    },
{% endif %}{% endfor %}  ],
};

const tableConfig = {
  permissionPrefix: "[[ permission_prefix ]]",
  table: { border: true },
  fetchData: [[ entity_name ]]API.getPage,
  deleteByIds: [[ entity_name ]]API.deleteByIds,
  toolbarActions: ["create", "delete"],
  tableTools: ["refresh", "columns", "search"],
  columns: [
    { type: "selection", width: 50 },
{% for c in columns %}{% if c.is_show_in_list %}    { prop: "[[ c.field_name ]]", label: "[[ c.field_comment ]]", minWidth: 120 },
{% endif %}{% endfor %}    {
      prop: "actions",
      label: "操作",
      cellType: "actions",
      actions: ["view", "edit", "delete"],
      fixed: "right",
      width: 200,
    },
  ],
};

const formConfig = {
  title: ({ mode }) => ({
    create: "新增[[ business_name ]]",
    edit: "编辑[[ business_name ]]",
    view: "查看[[ business_name ]]",
  })[mode],
  dialog: { width: 560 },
  /**
   * 保存记录，更新目标使用打开表单时的主键。
   */
  submitForm(data, { mode, id }) {
    if (mode === "create") return [[ entity_name ]]API.create(data);
    if (id === undefined) throw new Error("缺少记录 ID");
    return [[ entity_name ]]API.update(String(id), data);
  },
  fields: [
{% for c in columns %}{% if c.is_show_in_form and c.form_type_name != "HIDDEN" %}    {
      prop: "[[ c.field_name ]]",
      label: "[[ c.field_comment ]]",
{% if c.is_required %}      required: true,
{% endif %}{% if c.dict_type %}      type: "custom",
{% elif c.form_type_name == "SWITCH" %}      type: "switch",
{% if c.ts_type == "boolean" %}      initialValue: true,
      attrs: { activeValue: true, inactiveValue: false },
{% else %}      initialValue: 1,
      attrs: { activeValue: 1, inactiveValue: 0 },
{% endif %}{% elif c.form_type_name == "DATE" or c.form_type_name == "DATE_TIME" %}      type: "date-picker",
{% if c.form_type_name == "DATE_TIME" %}      attrs: { type: "datetime", valueFormat: "YYYY-MM-DD HH:mm:ss" },
{% else %}      attrs: { type: "date", valueFormat: "YYYY-MM-DD" },
{% endif %}{% elif c.form_type_name == "INPUT_NUMBER" %}      type: "input-number",
{% elif c.form_type_name == "SELECT" %}      type: "select",
      options: [],
{% elif c.form_type_name == "RADIO" %}      type: "radio",
      options: [],
{% elif c.form_type_name == "CHECK_BOX" %}      type: "checkbox",
      initialValue: [],
      options: [],
{% else %}      type: "input",
{% if c.form_type_name == "TEXT_AREA" %}      attrs: { type: "textarea", rows: 3 },
{% endif %}{% endif %}    },
{% endif %}{% endfor %}  ],
};

/**
 * 加载编辑和查看所需的详情。
 */
function loadFormData(row) {
  const rowKey = tableConfig.rowKey ?? "id";
  return [[ entity_name ]]API.getFormData(String(row[rowKey]));
}
</script>
"#;

// 渲染入口

/// 渲染全部模板（后端 Rust + 前端 Vue3）
///
/// `page_type`: "classic" | "crud"
/// `frontend_type`: "ts" | "js"
pub fn render(meta: &TableMeta, page_type: &str, frontend_type: &str) -> anyhow::Result<Vec<GeneratedFile>> {
    let mut out = Vec::new();

    // ── 后端（默认 {{ }} 分隔符）──
    {
        let mut tera = Tera::default();
        tera.add_raw_template("entity", ENTITY_TMPL)?;
        tera.add_raw_template("dto", DTO_TMPL)?;
        tera.add_raw_template("service", SERVICE_TMPL)?;
        tera.add_raw_template("mod", MOD_TMPL)?;
        let ctx = Context::from_serialize(meta)?;

        let base = format!("src/modules/{}/{}", meta.module_name, meta.entity_lower);
        out.push(GeneratedFile {
            path: "src/entity".into(),
            file_name: format!("{}.rs", meta.table_name),
            content: tera.render("entity", &ctx)?,
            scope: "backend".into(),
        });
        for (tpl, fname) in [("dto", "dto.rs"), ("service", "service.rs"), ("mod", "mod.rs")] {
            out.push(GeneratedFile {
                path: base.clone(),
                file_name: fname.into(),
                content: tera.render(tpl, &ctx)?,
                scope: "backend".into(),
            });
        }
    }

    // ── 前端（模板内 [[ ]] 预转为 {{ }}，Vue {{ }} 先保护后还原）──
    {
        let mut tera = Tera::default();
        let preprocess = |t: &str| -> String {
            t.replace("{{ ", "\x01O\x01")
             .replace(" }}", "\x01C\x01")
             .replace("[[ ", "{{ ")
             .replace(" ]]", " }}")
        };
        let postprocess = |t: &str| -> String {
            t.replace("\x01O\x01", "{{ ")
             .replace("\x01C\x01", " }}")
        };
        tera.add_raw_template("api", &preprocess(TS_API_TMPL))?;
        tera.add_raw_template("types", &preprocess(TS_TYPES_TMPL))?;
        tera.add_raw_template("classic", &preprocess(TS_CLASSIC_VUE_TMPL))?;
        tera.add_raw_template("crud", &preprocess(TS_CRUD_VUE_TMPL))?;
        tera.add_raw_template("js_api", &preprocess(JS_API_TMPL))?;
        tera.add_raw_template("js_classic", &preprocess(JS_CLASSIC_VUE_TMPL))?;
        tera.add_raw_template("js_crud", &preprocess(JS_CRUD_VUE_TMPL))?;
        let ctx = Context::from_serialize(meta)?;

        let fe = "vue3-element-admin";
        let api_path = format!("{}/src/api/{}/{}", fe, meta.module_name, meta.entity_kebab);
        let view_path = format!("{}/src/views/{}/{}", fe, meta.module_name, meta.entity_kebab);
        let is_js = frontend_type == "js";
        let is_crud = page_type.eq_ignore_ascii_case("crud");

        if is_js {
            out.push(GeneratedFile {
                path: api_path.clone(),
                file_name: "index.js".into(),
                content: postprocess(&tera.render("js_api", &ctx)?),
                scope: "frontend".into(),
            });
            out.push(GeneratedFile {
                path: view_path.clone(),
                file_name: "index.vue".into(),
                content: postprocess(&tera.render(if is_crud { "js_crud" } else { "js_classic" }, &ctx)?),
                scope: "frontend".into(),
            });
        } else {
            out.push(GeneratedFile {
                path: api_path.clone(),
                file_name: "index.ts".into(),
                content: postprocess(&tera.render("api", &ctx)?),
                scope: "frontend".into(),
            });
            out.push(GeneratedFile {
                path: api_path.clone(),
                file_name: "types.ts".into(),
                content: postprocess(&tera.render("types", &ctx)?),
                scope: "frontend".into(),
            });
            out.push(GeneratedFile {
                path: view_path.clone(),
                file_name: "index.vue".into(),
                content: postprocess(&tera.render(if is_crud { "crud" } else { "classic" }, &ctx)?),
                scope: "frontend".into(),
            });
        }
    }

    Ok(out)
}
