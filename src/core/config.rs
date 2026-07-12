#![allow(dead_code)]
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Settings {
    pub app: AppConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub security: SecurityConfig,
    #[serde(default)]
    pub minio: MinioConfig,
    #[serde(default)]
    pub mail: MailConfig,
    #[serde(default)]
    pub file: FileConfig,
    #[serde(default)]
    pub wx: WxConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
}

fn default_max_connections() -> u32 { 20 }
fn default_min_connections() -> u32 { 5 }

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    #[serde(default)]
    pub ignore_urls: String,
    #[serde(default)]
    pub unsecured_urls: String,
    pub session: SessionConfig,
}

impl SecurityConfig {
    pub fn ignore_urls_list(&self) -> Vec<String> {
        self.ignore_urls.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SessionConfig {
    #[serde(rename = "type")]
    pub r#type: String,
    pub access_token_time_to_live: i64,
    pub refresh_token_time_to_live: i64,
    pub jwt: JwtConfig,
    #[serde(default)]
    pub redis_token: RedisTokenConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret_key: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RedisTokenConfig {
    #[serde(default = "default_true")]
    pub allow_multi_login: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MinioConfig {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default)]
    pub secret_key: String,
    #[serde(default)]
    pub bucket: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MailConfig {
    #[serde(default)]
    pub server: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub from: String,
    #[serde(default = "default_true")]
    pub starttls: bool,
}

/// 微信小程序配置（配置项 `wx.miniapp`）
#[derive(Debug, Deserialize, Clone, Default)]
pub struct WxConfig {
    #[serde(default)]
    pub miniapp: WxMaConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct WxMaConfig {
    /// 小程序 AppID
    #[serde(default)]
    pub appid: String,
    /// 小程序 AppSecret
    #[serde(default)]
    pub secret: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FileConfig {
    /// 本地存储根目录
    #[serde(default = "default_file_storage")]
    pub storage_path: String,
    /// 访问基础路径（拼接日期目录与文件名即为可访问 URL）
    #[serde(default = "default_file_base_url")]
    pub base_url: String,
    /// 存储类型：local / minio（本期仅实现 local）
    #[serde(default = "default_file_type")]
    pub r#type: String,
}

fn default_file_storage() -> String { "./uploads".into() }
fn default_file_base_url() -> String { "/files".into() }
fn default_file_type() -> String { "local".into() }

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    /// 是否启用 IP 全局限流
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 默认每 IP+路径 滑动窗口内允许的最大请求数
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// 滑动窗口大小（秒）
    #[serde(default = "default_window")]
    pub window_seconds: u64,
    /// 严格路径的限流阈值（如登录、获取当前用户等高频接口）
    #[serde(default = "default_strict_limit")]
    pub strict_limit: u64,
    /// 逗号分隔的严格路径前缀列表
    #[serde(default = "default_strict_paths")]
    pub strict_paths: String,
}

fn default_limit() -> u64 { 1000 }
fn default_window() -> u64 { 60 }
fn default_strict_limit() -> u64 { 5 }
fn default_strict_paths() -> String { "/api/v1/users/me".into() }

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let figment = Figment::new()
            .merge(Toml::file("config/config.toml").nested())
            .merge(Toml::file("config/config-dev.toml").nested())
            .merge(Env::raw().split("__"));
        Ok(figment.extract()?)
    }

    pub fn is_dev(&self) -> bool {
        self.app.env == "dev"
    }
}
