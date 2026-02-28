// crates/core/src/security/auth/mod.rs
//! # 认证模块 (Authentication Module)
//!
//! **架构作用**:
//! 实现 09_auth.md 规定的 12-Factor Auth 体系：
//! - Argon2 密码哈希 (password)
//! - HS256 JWT Token 签发/验证 (jwt)
//! - 环境变量配置加载 (config)
//!
//! ## 模块组织
//! - `password`: Argon2 哈希生成与验证
//! - `jwt`: JWT Claims 定义, Token 签发与验证
//! - `config`: `AuthConfig` 环境变量加载

#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod jwt;
pub mod password;

// Re-exports (server-only: JWT requires ring which needs C compiler)
#[cfg(not(target_arch = "wasm32"))]
pub use self::config::AuthConfig;
#[cfg(not(target_arch = "wasm32"))]
pub use self::jwt::Claims;
