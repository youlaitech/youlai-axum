use serde::{Deserialize, Deserializer, Serializer};

/// 将 i64 序列化为 JSON 字符串，避免前端 JS 对超过 2^53 的雪花 ID 产生精度丢失。
pub fn i64_to_string<S: Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

/// PascalCase / camelCase → snake_case（如 MenuDemo → menu_demo）
pub fn snake_case(value: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in value.chars() {
        if ch == '-' || ch == '_' {
            out.push('_');
            prev_lower = false;
        } else if ch.is_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
            prev_lower = false;
        } else {
            out.push(ch);
            prev_lower = ch.is_lowercase() || ch.is_ascii_digit();
        }
    }
    out
}

/// PascalCase / snake_case → kebab-case（如 MenuDemo → menu-demo）；接口路径、前端目录与菜单路由共用此实现
pub fn kebab_case(value: &str) -> String {
    snake_case(value).replace('_', "-")
}

/// kebab-case → 英文复数（如 dict-item → dict-items、category → categories）；接口路径与前端目录共用
pub fn plural_kebab(value: &str) -> String {
    let word = kebab_case(value);
    if word.ends_with('s') || word.ends_with('x') || word.ends_with('z') || word.ends_with("ch") || word.ends_with("sh") {
        return format!("{word}es");
    }
    let chars: Vec<char> = word.chars().collect();
    if chars.len() > 1 && word.ends_with('y') && !"aeiou".contains(chars[chars.len() - 2]) {
        return format!("{}ies", &word[..word.len() - 1]);
    }
    format!("{word}s")
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

/// 反序列化可选字符串：兼容 JSON 数字、布尔与字符串，空串视为未传。
pub fn opt_string_from_any<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error as _;
    match Option::<serde_json::Value>::deserialize(d)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            let text = s.trim();
            Ok(if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            })
        }
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(serde_json::Value::Bool(b)) => Ok(Some(b.to_string())),
        Some(_) => Err(D::Error::custom("查询参数类型非法")),
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
