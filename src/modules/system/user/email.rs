//! 邮件发送服务：基于 lettre 发送 SMTP 邮件（验证码、通知）
//!
//! 配置缺失（server/username 为空）时降级为日志打印，
//! 避免无 SMTP 环境下注册/绑定流程被阻断。

use crate::config::MailConfig;

/// 发送邮箱验证码
pub async fn send_code(email: &str, code: &str, cfg: &MailConfig) {
    let subject = "【youlai-axum】邮箱验证码";
    let text = format!("您的验证码是：{}，5 分钟内有效。", code);
    if cfg.server.is_empty() || cfg.username.is_empty() {
        tracing::info!(
            "email code to {} (smtp not configured, print only): {}",
            email,
            code
        );
        return;
    }
    match send_smtp(email, subject, &text, cfg).await {
        Ok(()) => tracing::info!("email code sent to {}", email),
        Err(e) => tracing::warn!("send email to {} failed: {}", email, e),
    }
}

/// 通过 SMTP 发送纯文本邮件
async fn send_smtp(to: &str, subject: &str, body: &str, cfg: &MailConfig) -> anyhow::Result<()> {
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let email = Message::builder()
        .from(cfg.from.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())?;

    let mut builder = if cfg.starttls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.server)?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.server)?
    };
    builder = builder.port(cfg.port);
    if !cfg.username.is_empty() {
        builder = builder.credentials(Credentials::new(cfg.username.clone(), cfg.password.clone()));
    }
    builder.build().send(email).await?;
    Ok(())
}
