// crates/core/src/security/auth/config.rs
//! # 认证配置 (Auth Config)
//!
//! 从环境变量加载认证参数，遵循 12-Factor 原则。
//!
//! ## 环境变量 (09_auth.md §本章相关配置)
//! - `AUTH_SECRET`: JWT 签名密钥 (MUST >= 32 字节)
//! - `AUTH_USER`: 用户名 (默认 "admin")
//! - `AUTH_PASS`: Argon2 哈希后的密码
//! - `AUTH_ALLOW_ANONYMOUS_LOCALHOST`: 是否允许 localhost 免密

use anyhow::{anyhow, Result};
use tracing::warn;

/// 认证配置 (不可变，加载后冻结)
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT 签名密钥
    pub secret: String,
    /// 用户名 (默认 "admin")
    pub username: String,
    /// 密码的 Argon2 哈希 (PHC 格式)
    pub password_hash: String,
    /// 是否允许 localhost 免密访问
    pub allow_anonymous_localhost: bool,
    /// 当前 Token 版本 (修改密码后递增)
    pub token_version: u32,
}

impl AuthConfig {
    /// 从环境变量加载配置
    ///
    /// # 环境驱动行为
    /// - `DEVE_ENV` 默认为 `production`
    /// - 生产模式缺少密钥时必须失败关闭
    /// - 开发模式缺少密钥时才允许回退到 `dev_default()`
    pub fn from_env() -> Result<Self> {
        let env = std::env::var("DEVE_ENV").unwrap_or_else(|_| "production".to_string());
        let secret = std::env::var("AUTH_SECRET").ok();
        let password_hash = std::env::var("AUTH_PASS").ok();

        if secret.is_none() || password_hash.is_none() {
            if env == "development" {
                warn!("WARNING: Development mode with default credentials");
                return Self::dev_default();
            }
            return Err(anyhow!(
                "Production mode requires AUTH_SECRET and AUTH_PASS"
            ));
        }

        let secret = secret.expect("checked above");

        if secret.len() < 32 {
            return Err(anyhow!(
                "AUTH_SECRET must be >= 32 bytes (got {})",
                secret.len()
            ));
        }

        let username = std::env::var("AUTH_USER").unwrap_or_else(|_| "admin".into());
        let password_hash = password_hash.expect("checked above");

        let allow_anon = std::env::var("AUTH_ALLOW_ANONYMOUS_LOCALHOST")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let token_version = std::env::var("AUTH_TOKEN_VERSION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        Ok(Self {
            secret,
            username,
            password_hash,
            allow_anonymous_localhost: allow_anon,
            token_version,
        })
    }

    /// 创建用于开发/测试的默认配置
    ///
    /// 使用固定密钥和明文 "admin" 密码的哈希。
    /// **MUST NOT** 用于生产环境。
    pub fn dev_default() -> Result<Self> {
        let password_hash = super::password::hash_password("admin")?;
        Ok(Self {
            secret: "deve_dev_secret_key_32bytes_ok!!".into(),
            username: "admin".into(),
            password_hash,
            allow_anonymous_localhost: true,
            token_version: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_default() {
        let cfg = AuthConfig::dev_default().unwrap();
        assert_eq!(cfg.username, "admin");
        assert!(cfg.allow_anonymous_localhost);
        assert!(cfg.secret.len() >= 32);
    }
}
