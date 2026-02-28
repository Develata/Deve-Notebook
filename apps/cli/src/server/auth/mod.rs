// apps/cli/src/server/auth/mod.rs
//! # HTTP 认证模块
//!
//! 实现 09_auth.md 规定的认证端点与中间件:
//! - `handlers`: POST /api/auth/login, POST /api/auth/logout, GET /api/auth/me
//! - `middleware`: JWT Cookie 提取 + 验证中间件
//! - `brute_force`: IP 级暴力破解防护 (5 次/15 分钟)
//! - `headers`: 安全响应头中间件

pub mod brute_force;
pub mod handlers;
pub mod headers;
pub mod middleware;
