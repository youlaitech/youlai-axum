//! SSE 会话管理：user_id -> 多连接发送端，单进程内存模型。

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::modules::sse::topics::ONLINE_COUNT;

/// 一条 SSE 事件：事件名 + JSON 数据
#[derive(Clone, Debug, Serialize)]
pub struct SseEvent {
    pub event: String,
    pub data: serde_json::Value,
}

/// SSE 管理器：维护在线连接与会话表
pub struct MessageHub {
    /// user_id -> 该用户的多个连接发送端（多设备）
    sessions: RwLock<HashMap<String, Vec<(Uuid, mpsc::UnboundedSender<SseEvent>)>>>,
    /// 在线用户数（按 user_id 去重）
    online: AtomicUsize,
}

/// 连接生命周期守卫：drop 时自动从会话表移除并广播人数变化
pub struct SseGuard {
    manager: Arc<MessageHub>,
    user_id: i64,
    conn_id: Uuid,
}

impl MessageHub {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            online: AtomicUsize::new(0),
        }
    }

    /// 建立连接：注册发送端，返回接收端与守卫
    pub async fn connect(
        self: &Arc<Self>,
        user_id: i64,
    ) -> (mpsc::UnboundedReceiver<SseEvent>, SseGuard) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn_id = Uuid::new_v4();
        let key = user_id.to_string();

        let mut sessions = self.sessions.write().await;
        let is_new_user = !sessions.contains_key(&key);
        sessions.entry(key.clone()).or_default().push((conn_id, tx));

        if is_new_user {
            self.online.fetch_add(1, Ordering::SeqCst);
        }

        let guard = SseGuard {
            manager: Arc::clone(self),
            user_id,
            conn_id,
        };
        (rx, guard)
    }

    /// 移除指定连接；该 user_id 无连接时在线数 -1
    async fn disconnect(&self, user_id: i64, conn_id: Uuid) {
        let key = user_id.to_string();
        let mut sessions = self.sessions.write().await;
        let became_empty = if let Some(vec) = sessions.get_mut(&key) {
            vec.retain(|(id, _)| *id != conn_id);
            vec.is_empty()
        } else {
            false
        };
        if became_empty {
            sessions.remove(&key);
            self.online.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// 定向推送给指定用户的所有在线连接
    pub async fn send_to_user(&self, user_id: i64, event: &str, data: serde_json::Value) {
        let key = user_id.to_string();
        let sessions = self.sessions.read().await;
        if let Some(vec) = sessions.get(&key) {
            let evt = SseEvent {
                event: event.to_string(),
                data,
            };
            for (_, tx) in vec {
                let _ = tx.send(evt.clone());
            }
        }
    }

    /// 广播给所有在线连接
    pub async fn broadcast(&self, event: &str, data: serde_json::Value) {
        let sessions = self.sessions.read().await;
        let evt = SseEvent {
            event: event.to_string(),
            data,
        };
        for vec in sessions.values() {
            for (_, tx) in vec {
                let _ = tx.send(evt.clone());
            }
        }
    }

    /// 在线用户 ID 列表
    pub async fn get_online_users(&self) -> Vec<i64> {
        self.sessions
            .read()
            .await
            .keys()
            .filter_map(|k| k.parse::<i64>().ok())
            .collect()
    }

    /// 在线用户数
    pub fn online_count(&self) -> usize {
        self.online.load(Ordering::SeqCst)
    }
}

impl Drop for SseGuard {
    fn drop(&mut self) {
        // Drop 中不能 await，spawn 异步清理
        let manager = Arc::clone(&self.manager);
        let user_id = self.user_id;
        let conn_id = self.conn_id;
        tokio::spawn(async move {
            manager.disconnect(user_id, conn_id).await;
            let count = manager.online_count();
            manager
                .broadcast(ONLINE_COUNT, serde_json::json!(count))
                .await;
        });
    }
}

/// 心跳保活间隔（当前由 axum KeepAlive 承担）
#[allow(dead_code)]
pub const DEFAULT_HEARTBEAT: Duration = Duration::from_secs(30);
