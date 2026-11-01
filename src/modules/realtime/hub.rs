//! 实时连接会话管理：user_id -> 多连接发送端，单进程内存模型

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::modules::realtime::topics::EVENT_PRESENCE_SNAPSHOT;

/// 单条连接：连接标识、订阅主题与发送端
struct Session {
    conn_id: Uuid,
    topics: Vec<String>,
    tx: mpsc::UnboundedSender<String>,
}

/// 实时连接管理器：维护在线连接与主题订阅
pub struct MessageHub {
    /// user_id -> 该用户的多个连接（多设备）
    sessions: RwLock<HashMap<i64, Vec<Session>>>,
    /// 在线连接总数
    connections: AtomicUsize,
    /// 节点标识，进程内固定
    node_id: String,
}

/// 连接生命周期守卫：drop 时自动从会话表移除并广播在线状态
pub struct RealtimeGuard {
    manager: Arc<MessageHub>,
    user_id: i64,
    conn_id: Uuid,
}

impl MessageHub {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            connections: AtomicUsize::new(0),
            node_id: Uuid::new_v4().simple().to_string(),
        }
    }

    /// 建立连接：注册发送端，返回接收端与守卫
    pub async fn connect(
        self: &Arc<Self>,
        user_id: i64,
        topics: Vec<String>,
    ) -> (mpsc::UnboundedReceiver<String>, RealtimeGuard) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn_id = Uuid::new_v4();

        let mut sessions = self.sessions.write().await;
        sessions.entry(user_id).or_default().push(Session {
            conn_id,
            topics,
            tx,
        });
        self.connections.fetch_add(1, Ordering::SeqCst);

        let guard = RealtimeGuard {
            manager: Arc::clone(self),
            user_id,
            conn_id,
        };
        (rx, guard)
    }

    /// 移除指定连接
    async fn disconnect(&self, user_id: i64, conn_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        if let Some(vec) = sessions.get_mut(&user_id) {
            vec.retain(|s| s.conn_id != conn_id);
            if vec.is_empty() {
                sessions.remove(&user_id);
            }
        }
        self.connections.fetch_sub(1, Ordering::SeqCst);
    }

    /// 定向推送给指定用户的所有在线连接
    pub async fn send_to_user(&self, user_id: i64, msg_type: &str, payload: Value) {
        let key = user_id;
        let sessions = self.sessions.read().await;
        if let Some(vec) = sessions.get(&key) {
            let text = encode(msg_type, payload);
            for session in vec {
                let _ = session.tx.send(text.clone());
            }
        }
    }

    /// 广播给订阅了该主题的所有连接
    pub async fn broadcast(&self, topic: &str, msg_type: &str, payload: Value) {
        let sessions = self.sessions.read().await;
        let text = encode(msg_type, payload);
        for vec in sessions.values() {
            for session in vec {
                if !session.topics.iter().any(|t| t == topic) {
                    continue;
                }
                let _ = session.tx.send(text.clone());
            }
        }
    }

    /// 在线用户 ID 列表
    pub async fn get_online_users(&self) -> Vec<i64> {
        self.sessions.read().await.keys().copied().collect()
    }

    /// 在线用户数，按 user_id 去重
    pub async fn online_user_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// 在线连接总数
    pub fn online_connection_count(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// 节点标识
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 在线状态快照
    pub async fn presence_snapshot(&self) -> Value {
        let sessions = self.sessions.read().await;
        json!({
            "onlineUsers": sessions.len(),
            "onlineConnections": self.connections.load(Ordering::SeqCst),
            "nodeId": self.node_id,
            "observedAt": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        })
    }

    /// 广播在线状态快照
    pub async fn broadcast_presence(&self) {
        let snapshot = self.presence_snapshot().await;
        self.broadcast(
            crate::modules::realtime::topics::TOPIC_PRESENCE,
            EVENT_PRESENCE_SNAPSHOT,
            snapshot,
        )
        .await;
    }
}

impl Drop for RealtimeGuard {
    fn drop(&mut self) {
        // Drop 中不能 await，spawn 异步清理
        let manager = Arc::clone(&self.manager);
        let user_id = self.user_id;
        let conn_id = self.conn_id;
        tokio::spawn(async move {
            manager.disconnect(user_id, conn_id).await;
            manager.broadcast_presence().await;
        });
    }
}

/// 按协议封装服务端消息
pub fn encode(msg_type: &str, payload: Value) -> String {
    json!({
        "version": 1,
        "id": Uuid::new_v4().simple().to_string(),
        "type": msg_type,
        "payload": payload,
    })
    .to_string()
}
