// crates/core/src/security/auth.rs
//! # 认证管理器 (Auth Manager)
//!
//! 实现 Argon2 密码哈希验证和 JWT Token 生成。

use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub secret: String, // 用于 JWT 签名
}

pub struct AuthManager {
    #[allow(dead_code)] // 用于未来 JWT Token 生成
    secret: String,
}

impl AuthManager {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    /// 验证密码 (Argon2)
    ///
    /// 注意：在实际应用中，我们应该存储 Hash 后的密码。
    /// 这里简化为：如果提供了 Hash，则验证；如果未提供 Hash (第一次)，则生成并打印 Hash (用于配置)。
    ///
    /// 为了演示，我们假设 config 中存储的是明文密码（仅开发模式）或者 Hash。
    /// 实际上，Deve-Note 设计中 `AUTH_PASS` 环境变量应该是一个 Hash 字符串。
    pub fn verify_password(&self, password: &str, stored_hash: &str) -> Result<bool> {
        let parsed_hash =
            PasswordHash::new(stored_hash).map_err(|e| anyhow!("Invalid hash format: {}", e))?;

        match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 生成密码 Hash (工具函数)
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Hashing failed: {}", e))?;
        Ok(password_hash.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "my_secure_password";
        let hash = AuthManager::hash_password(password).expect("Hashing failed");

        let auth = AuthManager::new("secret".to_string());

        // Verify correct password
        assert!(auth
            .verify_password(password, &hash)
            .expect("Verification failed"));

        // Verify wrong password
        assert!(!auth
            .verify_password("wrong_password", &hash)
            .expect("Verification failed"));
    }
}
