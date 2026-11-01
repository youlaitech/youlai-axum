//! 实时连接与票据接口

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};
use crate::modules::auth::extractor::AuthUser;
use crate::modules::realtime::hub::{encode, MessageHub, RealtimeGuard};
use crate::modules::realtime::ticket::{consume_ticket, issue_ticket};
use crate::modules::realtime::topics::{
    EVENT_CONNECTION_READY, EVENT_HEARTBEAT_PING, EVENT_HEARTBEAT_PONG, HEARTBEAT_TIMEOUT_MS,
    PING_INTERVAL_SECS, SUPPORTED_TOPICS, TOPIC_NOTICE,
};
use crate::response::Result;
use crate::result_code::ResultCode;
use crate::state::AppState;

/// 申请握手票据的请求体
#[derive(Debug, Deserialize)]
pub struct CreateTicketRequest {
    /// 客户端实例标识
    #[serde(rename = "clientInstanceId", default)]
    pub client_instance_id: Option<String>,
    /// 客户端平台
    #[serde(default)]
    pub platform: Option<String>,
    /// 订阅主题，须包含 notice
    #[serde(default)]
    pub topics: Vec<String>,
}

/// 签发一次性握手票据
pub async fn create_ticket(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    headers: HeaderMap,
    Json(body): Json<CreateTicketRequest>,
) -> AppResult<Json<Result<Value>>> {
    if !body.topics.iter().any(|t| t == TOPIC_NOTICE) {
        return Err(AppError::Business(
            ResultCode::UserRequestParameterError,
            Some("订阅主题须包含 notice".into()),
        ));
    }

    for topic in &body.topics {
        if !SUPPORTED_TOPICS.contains(&topic.as_str()) {
            return Err(AppError::Business(
                ResultCode::UserRequestParameterError,
                Some(format!("不支持的订阅主题：{topic}")),
            ));
        }
    }

    let access_token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .trim()
                .trim_start_matches("Bearer ")
                .trim_start_matches("bearer ")
                .to_string()
        })
        .unwrap_or_default();

    let ticket = issue_ticket(
        &state.redis,
        user.user_id,
        &user.username,
        &access_token,
        &body.topics,
    )
    .await
    .map_err(|e| AppError::Business(ResultCode::SystemError, Some(e.to_string())))?;

    Ok(Json(Result::success(json!({ "ticket": ticket }))))
}

/// 建立实时连接，握手只接受一次性票据
pub async fn ws_connect(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Response {
    if params.len() != 1 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let Some(ticket) = params.get("ticket") else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Some(grant) = consume_ticket(&state.redis, ticket).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let hub = state.message_hub.clone();
    let (rx, guard) = hub.connect(grant.user_id, grant.topics.clone()).await;

    ws.on_upgrade(move |socket| handle_socket(socket, rx, guard, hub))
}

/// 处理已升级的连接：推送服务端消息、接收心跳响应
async fn handle_socket(
    socket: WebSocket,
    mut rx: mpsc::UnboundedReceiver<String>,
    guard: RealtimeGuard,
    hub: Arc<MessageHub>,
) {
    let (mut sender, mut receiver) = socket.split();
    let last_pong = Arc::new(AtomicU64::new(now_millis()));

    let ready = encode(
        EVENT_CONNECTION_READY,
        json!({ "heartbeatTimeoutMs": HEARTBEAT_TIMEOUT_MS }),
    );
    if sender.send(Message::Text(ready.into())).await.is_err() {
        return;
    }
    hub.broadcast_presence().await;

    // 服务端推送：业务消息与心跳
    let pong_slot = Arc::clone(&last_pong);
    let send_task = tokio::spawn(async move {
        let mut ping = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
        ping.tick().await;
        loop {
            tokio::select! {
                Some(text) = rx.recv() => {
                    if sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                _ = ping.tick() => {
                    if now_millis() - pong_slot.load(Ordering::SeqCst) > HEARTBEAT_TIMEOUT_MS {
                        break;
                    }
                    let message = encode(EVENT_HEARTBEAT_PING, json!({}));
                    if sender.send(Message::Text(message.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // 客户端上行，只处理心跳响应
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            if is_heartbeat_pong(&text) {
                last_pong.store(now_millis(), Ordering::SeqCst);
            }
        }
    }

    send_task.abort();
    drop(guard);
}

/// 查询在线状态
pub async fn presence(State(state): State<AppState>) -> AppResult<Json<Result<Value>>> {
    Ok(Json(Result::success(
        state.message_hub.presence_snapshot().await,
    )))
}

/// 判断是否为心跳响应
fn is_heartbeat_pong(text: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    value.get("version").and_then(|v| v.as_i64()) == Some(1)
        && value.get("type").and_then(|v| v.as_str()) == Some(EVENT_HEARTBEAT_PONG)
}

/// 当前毫秒时间戳
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
