use serde::{Deserialize, Deserializer, Serializer};

/// 将 i64 序列化为 JSON 字符串，避免前端 JS 对超过 2^53 的雪花 ID 产生精度丢失。
pub fn i64_to_string<S: Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

/// 反序列化可选 i64：兼容 JSON 数字、"123"、空串（视为未传）与 null。
pub fn opt_i64_from_any<'de, D>(d: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error as _;
    match Option::<serde_json::Value>::deserialize(d)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => n
            .as_i64()
            .map(Some)
            .ok_or_else(|| D::Error::custom("ID 应为整数")),
        Some(serde_json::Value::String(s)) => {
            let text = s.trim();
            if text.is_empty() {
                return Ok(None);
            }
            text.parse::<i64>()
                .map(Some)
                .map_err(|_| D::Error::custom("ID 格式非法"))
        }
        Some(_) => Err(D::Error::custom("ID 类型非法")),
    }
}

/// 反序列化可选 i64 列表：元素兼容 JSON 数字与字符串。
pub fn opt_vec_i64_from_any<'de, D>(d: D) -> Result<Option<Vec<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error as _;
    match Option::<Vec<serde_json::Value>>::deserialize(d)? {
        None => Ok(None),
        Some(items) => {
            let mut ids = Vec::with_capacity(items.len());
            for item in items {
                let id = match item {
                    serde_json::Value::Number(n) => n.as_i64(),
                    serde_json::Value::String(s) => s.trim().parse::<i64>().ok(),
                    _ => None,
                };
                ids.push(id.ok_or_else(|| D::Error::custom("ID 列表含非法值"))?);
            }
            Ok(Some(ids))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_i64_accepts_number_and_string() {
        assert_eq!(serde_json::from_str::<Opt>(r#"{"v":12}"#).unwrap().v, Some(12));
        assert_eq!(serde_json::from_str::<Opt>(r#"{"v":"12"}"#).unwrap().v, Some(12));
        assert_eq!(serde_json::from_str::<Opt>(r#"{"v":""}"#).unwrap().v, None);
        assert_eq!(serde_json::from_str::<Opt>(r#"{"v":null}"#).unwrap().v, None);
        assert_eq!(serde_json::from_str::<Opt>("{}").unwrap().v, None);
        assert!(serde_json::from_str::<Opt>(r#"{"v":"abc"}"#).is_err());
    }

    #[test]
    fn opt_vec_i64_accepts_mixed_elements() {
        #[derive(Deserialize)]
        struct ListOpt {
            #[serde(default, deserialize_with = "opt_vec_i64_from_any")]
            v: Option<Vec<i64>>,
        }
        assert_eq!(
            serde_json::from_str::<ListOpt>(r#"{"v":[1,"2"]}"#).unwrap().v,
            Some(vec![1, 2])
        );
        assert!(serde_json::from_str::<ListOpt>(r#"{"v":[1,"x"]}"#).is_err());
    }

    #[derive(Deserialize)]
    struct Opt {
        #[serde(default, deserialize_with = "opt_i64_from_any")]
        v: Option<i64>,
    }
}
