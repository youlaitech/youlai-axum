//! 代码生成器模块：表列表、生成配置 CRUD、代码预览与 zip 下载。

pub mod dto;
pub mod handler;
pub mod service;
pub mod templates;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

use self::handler::{ai_fill_config, delete_config, download, get_config, list_tables, preview, save_config};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/codegen/table", get(list_tables))
        .route(
            "/api/v1/codegen/{table_name}/config",
            get(get_config).post(save_config).delete(delete_config),
        )
        .route("/api/v1/codegen/{table_name}/preview", get(preview))
        .route("/api/v1/codegen/{table_name}/download", get(download))
        .route(
            "/api/v1/codegen/{table_name}/ai-config",
            axum::routing::post(ai_fill_config),
        )
}
