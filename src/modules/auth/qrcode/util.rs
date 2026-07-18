/// 昵称脱敏：保留首尾字符，中间以 * 填充
/// 长度 1：原样返回
/// 长度 2：首字 + *
/// 长度 ≥3：首字 + (n-2) 个 * + 末字
pub fn mask_nickname(nickname: &str) -> String {
    let chars: Vec<char> = nickname.chars().collect();
    let n = chars.len();
    if n <= 1 {
        return nickname.to_string();
    }
    if n == 2 {
        let mut s = String::new();
        s.push(chars[0]);
        s.push('*');
        return s;
    }
    let mut masked = String::new();
    masked.push(chars[0]);
    for _ in 0..n - 2 {
        masked.push('*');
    }
    masked.push(chars[n - 1]);
    masked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_various_lengths() {
        assert_eq!(mask_nickname("王"), "王");
        assert_eq!(mask_nickname("李雷"), "李*");
        assert_eq!(mask_nickname("欧阳娜娜"), "欧**娜");
    }
}
