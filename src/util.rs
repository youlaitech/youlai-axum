use serde::Serializer;

/// 将 i64 序列化为 JSON 字符串，避免前端 JS 对超过 2^53 的雪花 ID 产生精度丢失。
pub fn i64_to_string<S: Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}
