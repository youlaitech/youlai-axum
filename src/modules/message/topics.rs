//! SSE 事件名常量。

/// 在线用户数，data 为裸 int
pub const ONLINE_COUNT: &str = "online-users";

/// 字典变更，data 为 {"dictCode": String, "timestamp": i64}
pub const DICT: &str = "dict";

/// 系统消息，data 为 {"sender": String, "content": String, "timestamp": i64}
/// 预留事件名，当前后端暂无推送点
#[allow(dead_code)]
pub const SYSTEM: &str = "system";

/// 通知发布，data 为 {"id": i64, "title": String, "type": i16, "publishTime": String|null}
pub const NOTICE: &str = "notice";

/// 通知撤回，data 为 {"id": i64}
pub const NOTICE_REVOKE: &str = "notice-revoke";
