use crate::constants;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use captcha::Captcha as CaptchaGen;
use redis::aio::ConnectionManager;
use serde::Serialize;
use uuid::Uuid;

/// 验证码响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptchaResult {
    pub captcha_id: String,
    pub captcha_base64: String,
}

/// 生成图形验证码并存入 Redis（5 分钟有效期）
pub async fn generate(redis: &ConnectionManager) -> anyhow::Result<CaptchaResult> {
    // CaptchaGen 内部持有非 Send 的 RNG/字体（Rc<UnsafeCell>、dyn Font），
    // 必须在 await 之前 drop，否则返回的 Future 不是 Send，无法满足 axum Handler 约束。
    let (code, captcha_id, img_base64) = {
        let mut c = CaptchaGen::new();
        c.add_chars(constants::CAPTCHA_LENGTH as u32);
        let code = c.chars_as_string().to_lowercase();
        let captcha_id = Uuid::new_v4().to_string();
        let png = c
            .as_png()
            .ok_or_else(|| anyhow::anyhow!("生成验证码图片失败"))?;
        (code, captcha_id, STANDARD.encode(&png))
    };

    // 写入 Redis：captcha:{uuid} → code，TTL 300s
    let key = format!("{}:{}", constants::redis_key::CAPTCHA, &captcha_id);
    let mut r = redis.clone();
    let _: () = redis::cmd("SETEX")
        .arg(&key)
        .arg(constants::CAPTCHA_TTL)
        .arg(&code)
        .query_async(&mut r)
        .await?;

    Ok(CaptchaResult {
        captcha_id,
        captcha_base64: format!("data:image/png;base64,{img_base64}"),
    })
}

/// 校验验证码（一次消费，读后即删）
pub async fn verify(redis: &ConnectionManager, captcha_id: &str, code: &str) -> bool {
    let key = format!("{}:{}", constants::redis_key::CAPTCHA, captcha_id);
    let mut r = redis.clone();
    let stored: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut r)
        .await
        .ok()
        .flatten();
    // 无论成功失败都删除，防止暴力破解
    let _: () = redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut r)
        .await
        .unwrap_or(());
    stored.map_or(false, |s| s.to_lowercase() == code.to_lowercase())
}
