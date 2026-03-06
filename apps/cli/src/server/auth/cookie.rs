//! # Cookie 解析工具
//!
//! 提供精确 Cookie 名称匹配的提取函数，供 HTTP 中间件和 WebSocket 鉴权共用。
//!
//! ## Invariants
//! - 仅精确匹配 cookie 名称 (不匹配前缀)
//! - 例: cookie 名 "token" 不会匹配 "token_csrf" 或 "tokenBackup"

/// Auth cookie 名称
const COOKIE_NAME: &str = "token";

/// 从原始 Cookie header 字符串中精确提取指定名称的 cookie 值。
///
/// ## 算法
/// 遍历 `;` 分隔的 cookie 对，trim 后检查 `name=` 前缀（含等号），
/// 确保不会误匹配 `token_csrf` 等以 "token" 开头的 cookie。
pub fn extract_token_from_cookie_header(header: &str) -> Option<String> {
    let prefix = format!("{}=", COOKIE_NAME);
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(&prefix)
            && !value.is_empty()
        {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_only() {
        assert_eq!(
            extract_token_from_cookie_header("token=abc123"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_token_from_cookie_header("token_csrf=xyz; other=1"),
            None
        );
        assert_eq!(
            extract_token_from_cookie_header("session=x; token=jwt_here; lang=en"),
            Some("jwt_here".to_string())
        );
        assert_eq!(extract_token_from_cookie_header("token="), None);
        assert_eq!(extract_token_from_cookie_header("tokenBackup=bad"), None);
    }
}
