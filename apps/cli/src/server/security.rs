// apps/cli/src/server/security.rs
//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 05_network#server-ws-runtime
//!
//! # 安全密钥管理模块
//!
//! 管理宿主 Identity Key 的加载与生成。
//!
//! ## 不变量 (Invariants)
//! - Identity Key 必须始终存在 (首次启动时自动生成)
//! - Repo Key 可选，但一旦生成必须保持一致性

use deve_core::security::IdentityKeyPair;
use std::path::Path;
use std::sync::Arc;

/// 加载或生成 Identity Key
///
/// # 前置条件
/// - `host_key_dir` 必须是有效的宿主密钥目录路径
///
/// # 后置条件
/// - 返回的 `IdentityKeyPair` 已持久化到 `identity.key`
pub fn load_or_generate_identity_key(host_key_dir: &Path) -> anyhow::Result<Arc<IdentityKeyPair>> {
    let kp = deve_core::security::load_or_generate_identity_key_at(host_key_dir)?;
    tracing::info!(
        "IdentityKey ready at {:?}",
        host_key_dir.join("identity.key")
    );
    Ok(kp)
}
