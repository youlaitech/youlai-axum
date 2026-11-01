//! 实时推送主题与消息类型

/// 订阅主题，连接必须包含 notice
pub const TOPIC_NOTICE: &str = "notice";
/// 字典变更主题
pub const TOPIC_DICT: &str = "dict";
/// 在线状态主题
pub const TOPIC_PRESENCE: &str = "presence";

/// 当前支持的全部主题
pub const SUPPORTED_TOPICS: [&str; 3] = [TOPIC_NOTICE, TOPIC_DICT, TOPIC_PRESENCE];

/// 连接就绪，payload 为 heartbeatTimeoutMs
pub const EVENT_CONNECTION_READY: &str = "connection.ready";
/// 服务端心跳，payload 为空对象
pub const EVENT_HEARTBEAT_PING: &str = "heartbeat.ping";
/// 客户端心跳响应，payload 为 pingId
pub const EVENT_HEARTBEAT_PONG: &str = "heartbeat.pong";
/// 公告变更，payload 为 noticeId 与 action
pub const EVENT_NOTICE_CHANGED: &str = "notice.changed";
/// 字典变更，payload 为 dictCode
pub const EVENT_DICT_CHANGED: &str = "dict.changed";
/// 在线状态快照
pub const EVENT_PRESENCE_SNAPSHOT: &str = "presence.snapshot";

/// 心跳应答超时时间（毫秒），与前端协议保持一致
pub const HEARTBEAT_TIMEOUT_MS: u64 = 60_000;
/// 心跳发送间隔（秒）
pub const PING_INTERVAL_SECS: u64 = 25;
