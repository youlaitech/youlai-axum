use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::common::constants;
use crate::core::config::Settings;
use crate::framework::security::model::{AuthenticationToken, SecurityUserDetails};

/// 认证 Token 生命周期管理抽象
#[async_trait]
#[allow(dead_code)]
pub trait TokenManager: Send + Sync {
    async fn generate_token(&self, user: &SecurityUserDetails) -> AuthenticationToken;
    async fn parse_token(&self, token: &str) -> Option<SecurityUserDetails>;
    async fn validate_token(&self, token: &str) -> bool;
    async fn validate_refresh_token(&self, refresh_token: &str) -> bool;
    async fn refresh_token(&self, refresh_token: &str) -> Option<AuthenticationToken>;
    async fn invalidate_token(&self, token: &str);
    async fn invalidate_user_sessions(&self, user_id: i64);
}

pub type DynTokenManager = Box<dyn TokenManager>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,
    pub username: String,
    pub dept_id: Option<i64>,
    pub roles: Vec<String>,
    pub perms: Vec<String>,
    pub is_root: bool,
    pub data_scopes: Option<serde_json::Value>,
    pub token_type: String,
    pub exp: usize,
    pub iat: usize,
}

/// JWT 无状态 Token 管理器
pub struct JwtTokenManager {
    secret: String,
    access_ttl: i64,
    refresh_ttl: i64,
    #[allow(dead_code)]
    redis: tokio::sync::Mutex<ConnectionManager>,
}

impl JwtTokenManager {
    pub fn new(settings: &Settings, redis: ConnectionManager) -> Self {
        Self {
            secret: settings.security.session.jwt.secret_key.clone(),
            access_ttl: settings.security.session.access_token_time_to_live,
            refresh_ttl: settings.security.session.refresh_token_time_to_live,
            redis: tokio::sync::Mutex::new(redis),
        }
    }
}

#[async_trait]
impl TokenManager for JwtTokenManager {
    async fn generate_token(&self, user: &SecurityUserDetails) -> AuthenticationToken {
        let now = Utc::now().timestamp();
        let access_claims = JwtClaims {
            sub: user.user_id.to_string(), username: user.username.clone(),
            dept_id: user.dept_id, roles: user.roles.clone(), perms: user.perms.clone(),
            is_root: user.is_root,
            data_scopes: Some(serde_json::to_value(&user.data_scopes).unwrap_or_default()),
            token_type: constants::token_type::ACCESS.to_string(),
            exp: (now + self.access_ttl) as usize, iat: now as usize,
        };
        let access_token = encode(&Header::default(), &access_claims, &EncodingKey::from_secret(self.secret.as_bytes())).unwrap();

        let refresh_claims = JwtClaims { token_type: constants::token_type::REFRESH.to_string(), exp: (now + self.refresh_ttl) as usize, ..access_claims.clone() };
        let refresh_token = encode(&Header::default(), &refresh_claims, &EncodingKey::from_secret(self.secret.as_bytes())).unwrap();

        AuthenticationToken::new(access_token, refresh_token, self.access_ttl)
    }

    async fn parse_token(&self, token: &str) -> Option<SecurityUserDetails> {
        let data = decode::<JwtClaims>(token, &DecodingKey::from_secret(self.secret.as_bytes()), &Validation::new(Algorithm::HS256)).ok()?;
        let claims = data.claims;
        if claims.token_type != constants::token_type::ACCESS { return None; }
        let data_scopes: Vec<crate::framework::security::model::RoleDataScope> = serde_json::from_value(claims.data_scopes.unwrap_or_default()).unwrap_or_default();
        Some(SecurityUserDetails {
            user_id: claims.sub.parse().ok()?, username: claims.username, dept_id: claims.dept_id,
            roles: claims.roles.into_iter().collect(), perms: claims.perms, data_scopes, is_root: claims.is_root,
            ..Default::default()
        })
    }

    async fn validate_token(&self, token: &str) -> bool { self.parse_token(token).await.is_some() }

    async fn validate_refresh_token(&self, refresh_token: &str) -> bool {
        decode::<JwtClaims>(refresh_token, &DecodingKey::from_secret(self.secret.as_bytes()), &Validation::new(Algorithm::HS256)).is_ok()
    }

    async fn refresh_token(&self, refresh_token: &str) -> Option<AuthenticationToken> {
        let claims = decode::<JwtClaims>(refresh_token, &DecodingKey::from_secret(self.secret.as_bytes()), &Validation::new(Algorithm::HS256)).ok()?.claims;
        if claims.token_type != constants::token_type::REFRESH { return None; }
        let user = SecurityUserDetails { user_id: claims.sub.parse().ok()?, username: claims.username, dept_id: claims.dept_id, roles: claims.roles.into_iter().collect(), perms: claims.perms, is_root: claims.is_root, ..Default::default() };
        Some(self.generate_token(&user).await)
    }

    async fn invalidate_token(&self, _token: &str) {}

    async fn invalidate_user_sessions(&self, user_id: i64) {
        let mut r = self.redis.lock().await;
        let _: () = redis::cmd("DEL").arg(format!("{}:{}", constants::redis_key::JWT_VERSION, user_id)).query_async(&mut *r).await.unwrap_or(());
    }
}

/// Redis 有状态 Token 管理器：UUID 令牌 + Redis 存储会话快照，支持主动踢人
pub struct RedisTokenManager {
    access_ttl: i64, refresh_ttl: i64, allow_multi_login: bool, redis: tokio::sync::Mutex<ConnectionManager>,
}

impl RedisTokenManager {
    pub fn new(settings: &Settings, redis: ConnectionManager) -> Self {
        Self { access_ttl: settings.security.session.access_token_time_to_live, refresh_ttl: settings.security.session.refresh_token_time_to_live, allow_multi_login: settings.security.session.redis_token.allow_multi_login, redis: tokio::sync::Mutex::new(redis) }
    }
}

#[async_trait]
impl TokenManager for RedisTokenManager {
    async fn generate_token(&self, user: &SecurityUserDetails) -> AuthenticationToken {
        let mut r = self.redis.lock().await;
        if !self.allow_multi_login {
            let sessions_key = format!("{}:{}", constants::redis_key::USER_SESSIONS, user.user_id);
            let old_tokens: Vec<String> = redis::cmd("SMEMBERS").arg(&sessions_key).query_async(&mut *r).await.unwrap_or_default();
            for t in &old_tokens {
                let _: () = redis::cmd("DEL").arg(format!("{}:{}", constants::redis_key::AUTH_TOKEN, t)).query_async(&mut *r).await.unwrap_or(());
            }
            let _: () = redis::cmd("DEL").arg(&sessions_key).query_async(&mut *r).await.unwrap_or(());
        }

        let access_token = Uuid::new_v4().to_string();
        let refresh_token = Uuid::new_v4().to_string();
        let user_json = serde_json::to_string(user).unwrap();

        let _: () = redis::cmd("SETEX").arg(format!("{}:{}", constants::redis_key::AUTH_TOKEN, &access_token)).arg(self.access_ttl).arg(&user_json).query_async(&mut *r).await.unwrap();
        let _: () = redis::cmd("SETEX").arg(format!("{}:{}", constants::redis_key::AUTH_TOKEN, &refresh_token)).arg(self.refresh_ttl).arg(&user_json).query_async(&mut *r).await.unwrap();
        let _: () = redis::cmd("SADD").arg(format!("{}:{}", constants::redis_key::USER_SESSIONS, user.user_id)).arg(&access_token).query_async(&mut *r).await.unwrap_or(());

        AuthenticationToken::new(access_token, refresh_token, self.access_ttl)
    }

    async fn parse_token(&self, token: &str) -> Option<SecurityUserDetails> {
        let mut r = self.redis.lock().await;
        let key = format!("{}:{}", constants::redis_key::AUTH_TOKEN, token);
        let json: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut *r).await.ok().flatten();
        json.and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn validate_token(&self, token: &str) -> bool { self.parse_token(token).await.is_some() }
    async fn validate_refresh_token(&self, refresh_token: &str) -> bool { self.parse_token(refresh_token).await.is_some() }

    async fn refresh_token(&self, refresh_token: &str) -> Option<AuthenticationToken> {
        let user = self.parse_token(refresh_token).await?;
        let mut r = self.redis.lock().await;
        let _: () = redis::cmd("DEL").arg(format!("{}:{}", constants::redis_key::AUTH_TOKEN, refresh_token)).query_async(&mut *r).await.unwrap_or(());
        drop(r);
        Some(self.generate_token(&user).await)
    }

    async fn invalidate_token(&self, token: &str) {
        let mut r = self.redis.lock().await;
        let _: () = redis::cmd("DEL").arg(format!("{}:{}", constants::redis_key::AUTH_TOKEN, token)).query_async(&mut *r).await.unwrap_or(());
    }

    async fn invalidate_user_sessions(&self, user_id: i64) {
        let mut r = self.redis.lock().await;
        let key = format!("{}:{}", constants::redis_key::USER_SESSIONS, user_id);
        let tokens: Vec<String> = redis::cmd("SMEMBERS").arg(&key).query_async(&mut *r).await.unwrap_or_default();
        for t in &tokens {
            let _: () = redis::cmd("DEL").arg(format!("{}:{}", constants::redis_key::AUTH_TOKEN, t)).query_async(&mut *r).await.unwrap_or(());
        }
        let _: () = redis::cmd("DEL").arg(&key).query_async(&mut *r).await.unwrap_or(());
    }
}

/// 根据配置创建对应类型的 TokenManager
pub fn create_token_manager(settings: &Settings, redis: ConnectionManager) -> DynTokenManager {
    match settings.security.session.r#type.as_str() {
        "jwt" => Box::new(JwtTokenManager::new(settings, redis)),
        "redis-token" => Box::new(RedisTokenManager::new(settings, redis)),
        _ => panic!("unsupported session type: {}", settings.security.session.r#type),
    }
}
