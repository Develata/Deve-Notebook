// crates/core/src/security/auth/password.rs
//! # Argon2 密码哈希 (Password Hashing)
//!
//! 提供密码哈希生成与验证。
//!
//! ## 不变量 (Invariants)
//! - 所有密码在存储前必须经过 Argon2id 哈希处理
//! - 验证过程为常数时间比较，防止时序攻击

use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

/// 验证密码是否匹配已存储的 Argon2 哈希
///
/// # 前置条件
/// - `stored_hash` 必须是有效的 PHC 格式字符串
///
/// # 后置条件
/// - 返回 Ok(true) 当且仅当密码匹配
pub fn verify_password(password: &str, stored_hash: &str) -> Result<bool> {
    let parsed =
        PasswordHash::new(stored_hash).map_err(|e| anyhow!("Invalid hash format: {}", e))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// 生成密码的 Argon2 哈希 (PHC 格式)
///
/// # 后置条件
/// - 返回的字符串可直接传入 `verify_password`
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Hashing failed: {}", e))?;
    Ok(hash.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "my_secure_password";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }
}
