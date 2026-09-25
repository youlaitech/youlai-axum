//! youlai-axum 库入口，声明模块树并导出 build_router 与 run

pub mod ai;
pub mod app;
pub mod config;
pub mod constants;
pub mod core;
pub mod entity;
pub mod error;
pub mod modules;
pub mod openapi;
pub mod pagination;
pub mod rate_limit;
pub mod response;
pub mod result_code;
pub mod state;
pub mod util;

pub use app::{build_router, run};
