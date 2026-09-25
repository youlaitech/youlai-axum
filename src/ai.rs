//! AI 辅助能力：OpenAI 兼容对话接口与提示词加载

use crate::config::AiConfig;
use crate::error::{AppError, AppResult};
use crate::result_code::ResultCode;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

static PROMPTS: OnceLock<HashMap<String, String>> = OnceLock::new();

/// 读取提示词原文，名称为 prompts 目录下的相对路径（如 form/system.md）
pub fn prompt(name: &str) -> AppResult<&'static str> {
    let prompts = PROMPTS.get_or_init(load_prompts);
    prompts
        .get(name)
        .map(|text| text.as_str())
        .ok_or_else(|| {
            AppError::Business(
                ResultCode::UserRequestParameterError,
                Some(format!("提示词不存在：{name}")),
            )
        })
}

/// 发送提示词并返回模型输出文本
pub async fn chat(
    http: &reqwest::Client,
    config: &AiConfig,
    system_prompt: &str,
    user_prompt: &str,
) -> AppResult<String> {
    if !config.enabled {
        return Err(AppError::Business(
            ResultCode::UserRequestParameterError,
            Some("AI 功能未开启，请配置 ai.enabled 与 api_key".into()),
        ));
    }

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let body = json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "response_format": { "type": "json_object" }
    });

    let response = http
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            AppError::Business(
                ResultCode::UserRequestParameterError,
                Some(format!("AI 调用失败：{e}")),
            )
        })?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        tracing::error!("AI 调用失败 {status}: {text}");
        return Err(AppError::Business(
            ResultCode::UserRequestParameterError,
            Some(format!("AI 调用失败：{status}")),
        ));
    }

    let value: Value = serde_json::from_str(&text).map_err(|e| {
        AppError::Business(
            ResultCode::UserRequestParameterError,
            Some(format!("AI 响应解析失败：{e}")),
        )
    })?;
    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default();

    Ok(strip_code_fence(content))
}

/// 发送提示词并把 JSON 输出解析为数组
pub async fn chat_json_list(
    http: &reqwest::Client,
    config: &AiConfig,
    system_prompt: &str,
    user_prompt: &str,
) -> AppResult<Value> {
    let content = chat(http, config, system_prompt, user_prompt).await?;
    serde_json::from_str(&content).map_err(|e| {
        AppError::Business(
            ResultCode::UserRequestParameterError,
            Some(format!("AI 输出不是合法 JSON：{e}")),
        )
    })
}

/// 去掉模型输出里可能包裹的 json 代码围栏
fn strip_code_fence(content: &str) -> String {
    let text = content.trim();
    if !text.starts_with("```") {
        return text.to_string();
    }
    match (text.find('\n'), text.rfind("```")) {
        (Some(start), Some(end)) if end > start => text[start + 1..end].trim().to_string(),
        _ => text.to_string(),
    }
}

/// 扫描 prompts 目录并载入全部 md 提示词
fn load_prompts() -> HashMap<String, String> {
    let mut prompts = HashMap::new();
    let root = Path::new("prompts");
    collect_prompts(root, root, &mut prompts);
    prompts
}

fn collect_prompts(root: &Path, dir: &Path, prompts: &mut HashMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_prompts(root, &path, prompts);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            if let (Ok(content), Ok(relative)) = (std::fs::read_to_string(&path), path.strip_prefix(root)) {
                let key = relative.to_string_lossy().replace('\\', "/");
                prompts.insert(key, content.trim().to_string());
            }
        }
    }
}
