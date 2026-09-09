#![allow(dead_code)]
use serde::Deserialize;
use toml::Value;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Settings {
    pub app: AppConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub security: SecurityConfig,
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

fn default_max_connections() -> u32 {
    20
}
fn default_min_connections() -> u32 {
    5
}

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
        self.ignore_urls
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
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

fn default_true() -> bool {
    true
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

/// 文件存储配置：支持 S3(RustFS) / 阿里云 OSS / 本地磁盘三种后端
#[derive(Debug, Deserialize, Clone, Default)]
pub struct FileConfig {
    /// 存储类型：s3 | aliyun | local
    #[serde(rename = "type", default = "default_file_type")]
    pub r#type: String,
    /// 上传限制
    #[serde(default)]
    pub upload: FileUploadConfig,
    /// S3(RustFS) 对象存储
    #[serde(default)]
    pub s3: FileS3Config,
    /// 阿里云 OSS 对象存储
    #[serde(default)]
    pub aliyun: FileAliyunConfig,
    /// 本地存储
    #[serde(default)]
    pub local: FileLocalConfig,
}

/// 上传限制
#[derive(Debug, Deserialize, Clone)]
pub struct FileUploadConfig {
    /// 单文件大小上限（MB），0 表示不限制
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
    /// 允许的文件扩展名白名单（逗号分隔，空表示不限制）
    #[serde(default)]
    pub allowed_extensions: String,
}

impl Default for FileUploadConfig {
    fn default() -> Self {
        Self {
            max_file_size_mb: default_max_file_size_mb(),
            allowed_extensions: String::new(),
        }
    }
}

/// S3(RustFS) 对象存储配置
#[derive(Debug, Deserialize, Clone, Default)]
pub struct FileS3Config {
    /// 服务地址（如 http://host:9000）
    #[serde(default)]
    pub endpoint: String,
    /// 访问凭据
    #[serde(default)]
    pub access_key: String,
    /// 凭据密钥
    #[serde(default)]
    pub secret_key: String,
    /// 存储桶名称
    #[serde(default)]
    pub bucket: String,
    /// 自定义域名（配置后文件 URL 走域名，留空则用 endpoint）
    #[serde(default)]
    pub domain: String,
}

/// 阿里云 OSS 对象存储配置
#[derive(Debug, Deserialize, Clone, Default)]
pub struct FileAliyunConfig {
    /// OSS 服务地址
    #[serde(default)]
    pub endpoint: String,
    /// 阿里云 RAM AccessKeyId
    #[serde(default)]
    pub access_key_id: String,
    /// 阿里云 RAM AccessKeySecret
    #[serde(default)]
    pub access_key_secret: String,
    /// 存储桶名称
    #[serde(default)]
    pub bucket: String,
}

/// 本地存储配置
#[derive(Debug, Deserialize, Clone)]
pub struct FileLocalConfig {
    /// 存储根目录
    #[serde(default = "default_file_storage")]
    pub path: String,
    /// 访问基础路径（拼接日期目录与文件名即为可访问 URL）
    #[serde(default = "default_file_base_url")]
    pub base_url: String,
}

impl Default for FileLocalConfig {
    fn default() -> Self {
        Self {
            path: default_file_storage(),
            base_url: default_file_base_url(),
        }
    }
}

fn default_file_storage() -> String {
    "./uploads".into()
}
fn default_file_base_url() -> String {
    "/files".into()
}
fn default_file_type() -> String {
    "s3".into()
}
fn default_max_file_size_mb() -> u64 {
    50
}

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

fn default_limit() -> u64 {
    1000
}
fn default_window() -> u64 {
    60
}
fn default_strict_limit() -> u64 {
    5
}
fn default_strict_paths() -> String {
    "/api/v1/auth/login".into()
}

impl Settings {
    /// 加载配置：config.toml（公共兜底）→ config-dev.toml（开发覆盖）→ config-{env}.toml（生产等）
    ///
    /// 环境通过 `APP__ENV` 或 `APP_ENV` 指定，默认 `dev`。
    /// 例：`APP__ENV=prod cargo run` → 依次加载 config.toml + config-dev.toml + config-prod.toml
    ///
    /// 注意：未使用 figment 的 merge 链（figment 0.10 在 Windows 上 merge 行为异常），
    /// 改为手动深合并 toml::Table 后再反序列化。
    pub fn load() -> anyhow::Result<Self> {
        let profile = std::env::var("APP__ENV")
            .or_else(|_| std::env::var("APP_ENV"))
            .unwrap_or_else(|_| "dev".into());

        // 1) 手动深合并 TOML 文件
        let mut base: toml::Table = toml::from_str(
            &std::fs::read_to_string("config/config.toml")?
        )?;
        let dev: toml::Table = toml::from_str(
            &std::fs::read_to_string("config/config-dev.toml")?
        )?;
        deep_merge(&mut base, &dev);

        if profile != "dev" {
            let prod: toml::Table = toml::from_str(
                &std::fs::read_to_string(format!("config/config-{profile}.toml"))?
            )?;
            deep_merge(&mut base, &prod);
        }

        // 2) 从合并后的 TOML 直接反序列化（绕过 figment）
        Ok(toml::from_str(&toml::to_string(&base)?)?)
    }

    pub fn is_dev(&self) -> bool {
        self.app.env == "dev"
    }
}

/// 递归合并两个 toml::Table：overlay 的键覆盖/新增到 base 中
fn deep_merge(base: &mut toml::Table, overlay: &toml::Table) {
    for (key, value) in overlay {
        match (base.get_mut(key), value) {
            (Some(Value::Table(b)), Value::Table(o)) => deep_merge(b, o),
            (_, v) => {
                base.insert(key.clone(), v.clone());
            }
        }
    }
}
