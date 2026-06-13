// crates/core/src/security/auth/jwt.rs
//! plan_ref:
//!   - 08_auth#jwt-cookie-contract
//!
//! # JWT Token 管理
//!
//! 实现 HS256 JWT 的签发与验证。
//!
//! ## Invariants
//! - Token 有效期严格为 24 小时
//! - `ver` 字段用于 Token Revocation（密码变更后递增）
//! - 签名密钥来自环境变量 `AUTH_SECRET`，禁止硬编码

use anyhow::{Result, anyhow};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT 有效期: 24 小时 (秒)
const TOKEN_LIFETIME_SECS: i64 = 86_400;

/// JWT Payload (Claims)
///
/// 遵循 `08_auth.md` 规范:
/// ```json
/// { "sub": "<current-user>", "iat": ..., "exp": ..., "ver": 1, "sid": "..." }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    /// Subject — 当前认证用户。
    pub sub: String,
    /// Issued At (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Token Version — 用于 Revocation
    pub ver: u32,
    /// Per-login session id. Optional so tokens minted before this field still validate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
}

/// 签发 JWT Token
///
/// # 前置条件
/// - `secret` 长度 >= 32 字节 (HS256 安全要求)
/// - `subject` 为已认证用户名
/// - `token_version` 为当前有效版本号
///
/// # 后置条件
/// - 返回的 Token 在 24 小时内有效
pub fn issue_token(secret: &str, subject: &str, token_version: u32) -> Result<String> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: subject.to_string(),
        iat: now,
        exp: now + TOKEN_LIFETIME_SECS,
        ver: token_version,
        sid: Some(uuid::Uuid::new_v4().simple().to_string()),
    };
    let key = EncodingKey::from_secret(secret.as_bytes());
    jsonwebtoken::encode(&Header::default(), &claims, &key)
        .map_err(|e| anyhow!("JWT encode failed: {}", e))
}

/// 验证并解码 JWT Token
///
/// # 前置条件
/// - `token` 为 Base64 编码的 JWT 字符串
///
/// # 后置条件
/// - 成功时返回解码后的 Claims
/// - 过期 / 签名无效 / 版本不匹配均返回 Err
pub fn validate_token(secret: &str, token: &str, current_version: u32) -> Result<Claims> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["sub", "exp", "iat"]);

    let data = jsonwebtoken::decode::<Claims>(token, &key, &validation)
        .map_err(|e| anyhow!("JWT validation failed: {}", e))?;

    // Token Revocation: 版本号必须匹配
    if data.claims.ver != current_version {
        return Err(anyhow!("Token revoked (ver mismatch)"));
    }
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_and_validate() {
        let secret = "test_secret_key_at_least_32_bytes_long!";
        let token = issue_token(secret, "admin", 1).unwrap();
        let claims = validate_token(secret, &token, 1).unwrap();
        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.ver, 1);
        assert!(claims.sid.is_some());
    }

    #[test]
    fn issue_token_preserves_subject() {
        let secret = "test_secret_key_at_least_32_bytes_long!";
        let token = issue_token(secret, "alice", 7).unwrap();
        let claims = validate_token(secret, &token, 7).unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.ver, 7);
    }

    #[test]
    fn issue_token_mints_unique_session_id_per_login() {
        let secret = "test_secret_key_at_least_32_bytes_long!";

        let first = issue_token(secret, "admin", 1).unwrap();
        let second = issue_token(secret, "admin", 1).unwrap();
        let first_claims = validate_token(secret, &first, 1).unwrap();
        let second_claims = validate_token(secret, &second, 1).unwrap();

        assert_ne!(first, second);
        assert_ne!(first_claims.sid, second_claims.sid);
    }

    #[test]
    fn validate_token_accepts_legacy_payload_without_session_id() {
        let secret = "test_secret_key_at_least_32_bytes_long!";
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: "admin".to_string(),
            iat: now,
            exp: now + TOKEN_LIFETIME_SECS,
            ver: 1,
            sid: None,
        };
        let token = jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let decoded = validate_token(secret, &token, 1).unwrap();

        assert_eq!(decoded.sub, "admin");
        assert_eq!(decoded.sid, None);
    }

    #[test]
    fn test_revoked_token() {
        let secret = "test_secret_key_at_least_32_bytes_long!";
        let token = issue_token(secret, "admin", 1).unwrap();
        let result = validate_token(secret, &token, 2); // ver mismatch
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret() {
        let token = issue_token("secret_a_32_bytes_long_xxxxxxxx!", "admin", 1).unwrap();
        let result = validate_token("secret_b_32_bytes_long_xxxxxxxx!", &token, 1);
        assert!(result.is_err());
    }
}
