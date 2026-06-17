// crates/core/src/security/auth/config.rs
//! plan_ref:
//!   - 08_auth#auth-config
//!
//! # 认证配置 (Auth Config)
//!
//! 从环境变量加载认证参数，遵循 12-Factor 原则。
//!
//! ## 环境变量 (09_auth#auth-config)
//! - `AUTH_SECRET`: JWT 签名密钥 (MUST >= 32 字节)
//! - `AUTH_USER`: 用户名 (默认 "admin")
//! - `AUTH_PASS`: Argon2 哈希后的密码
//! - `AUTH_ALLOW_ANONYMOUS_LOCALHOST`: 是否允许 localhost 免密
//! - `DEVE_ENV`: 部署环境 (`production` / `development`)

use super::password;
use anyhow::{Result, anyhow};

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
    /// 从环境变量加载配置；默认按生产模式 fail-closed。
    /// 开发模式触发条件：`DEVE_ENV=development`（必须显式设置）。
    /// 注意：不再自动根据 debug/release 构建模式切换。
    pub fn from_env() -> Result<Self> {
        let env = std::env::var("DEVE_ENV").unwrap_or_else(|_| "production".to_string());
        let is_dev_mode = env.trim().eq_ignore_ascii_case("development");
        let secret = std::env::var("AUTH_SECRET").ok();
        let password_hash = std::env::var("AUTH_PASS").ok();

        let (secret, password_hash) = match (secret, password_hash) {
            (Some(secret), Some(password_hash)) => (secret, password_hash),
            _ if is_dev_mode => {
                tracing::warn!(
                    "DEVE_ENV=development detected → using dev defaults (INSECURE for production)"
                );
                return Self::dev_default();
            }
            _ => {
                return Err(anyhow!(
                    "ERROR: Production mode requires AUTH_SECRET and AUTH_PASS"
                ));
            }
        };

        if secret.len() < 32 {
            return Err(anyhow!(
                "AUTH_SECRET must be >= 32 bytes (got {})",
                secret.len()
            ));
        }

        let username = std::env::var("AUTH_USER").unwrap_or_else(|_| "admin".into());
        password::validate_argon2_phc(&password_hash)
            .map_err(|err| anyhow!("AUTH_PASS must be a valid Argon2 PHC hash: {}", err))?;

        let allow_anon = parse_allow_anonymous_localhost_env(is_dev_mode)?;

        let token_version = parse_token_version_env()?;

        Ok(Self {
            secret,
            username,
            password_hash,
            allow_anonymous_localhost: allow_anon,
            token_version,
        })
    }

    /// 创建用于显式开发模式的默认配置，禁止作为生产环境静默回退。
    /// 开发模式也默认要求真实登录；localhost 免密必须显式通过环境变量开启。
    pub fn dev_default() -> Result<Self> {
        let password_hash = super::password::hash_password("admin")?;
        Ok(Self {
            secret: "deve_dev_secret_key_32bytes_ok!!".into(),
            username: "admin".into(),
            password_hash,
            allow_anonymous_localhost: env_flag("AUTH_ALLOW_ANONYMOUS_LOCALHOST", false),
            token_version: 1,
        })
    }
}

/// 解析布尔环境变量，供认证契约统一读取环境驱动开关。
fn env_flag(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn parse_allow_anonymous_localhost_env(is_dev_mode: bool) -> Result<bool> {
    let allow_anon = env_flag("AUTH_ALLOW_ANONYMOUS_LOCALHOST", false);
    if allow_anon && !is_dev_mode {
        return Err(anyhow!(
            "AUTH_ALLOW_ANONYMOUS_LOCALHOST requires DEVE_ENV=development"
        ));
    }
    Ok(allow_anon)
}

fn parse_token_version_env() -> Result<u32> {
    match std::env::var("AUTH_TOKEN_VERSION") {
        Ok(value) => value
            .parse::<u32>()
            .map_err(|_| anyhow!("AUTH_TOKEN_VERSION must be a valid u32 integer: {}", value)),
        Err(std::env::VarError::NotPresent) => Ok(1),
        Err(err) => Err(anyhow!("Failed to read AUTH_TOKEN_VERSION: {}", err)),
    }
}

#[cfg(test)]
mod tests;
