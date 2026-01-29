// apps/cli/src/server/security.rs
//! # 安全密钥管理模块
//!
//! 管理 Identity Key 和 Repo Key 的加载与生成。
//!
//! ## 不变量 (Invariants)
//! - Identity Key 必须始终存在 (首次启动时自动生成)
//! - Repo Key 可选，但一旦生成必须保持一致性

use deve_core::security::{IdentityKeyPair, RepoKey};
use std::path::Path;
use std::sync::Arc;

/// 加载或生成 Identity Key
///
/// # 前置条件
/// - `deve_dir` 必须是有效的 `.deve` 目录路径
///
/// # 后置条件
/// - 返回的 `IdentityKeyPair` 已持久化到 `identity.key`
pub fn load_or_generate_identity_key(deve_dir: &Path) -> anyhow::Result<Arc<IdentityKeyPair>> {
    let key_pair_path = deve_dir.join("identity.key");

    if key_pair_path.exists() {
        // 从文件加载已有密钥
        let bytes = std::fs::read(&key_pair_path)?;
        match IdentityKeyPair::from_bytes(&bytes) {
            Some(kp) => {
                tracing::info!("Loaded IdentityKey from {:?}", key_pair_path);
                Ok(Arc::new(kp))
            }
            None => {
                tracing::warn!("Invalid identity.key file, regenerating...");
                let kp = IdentityKeyPair::generate();
                std::fs::write(&key_pair_path, kp.to_bytes())?;
                Ok(Arc::new(kp))
            }
        }
    } else {
        // 生成新密钥并保存
        let kp = IdentityKeyPair::generate();
        std::fs::write(&key_pair_path, kp.to_bytes())?;
        tracing::info!("Generated and saved new IdentityKey to {:?}", key_pair_path);
        Ok(Arc::new(kp))
    }
}

/// 加载或生成 Repo Key (共享密钥)
///
/// # 前置条件
/// - `deve_dir` 必须是有效的 `.deve` 目录路径
///
/// # 后置条件
/// - 如果成功，返回的 `RepoKey` 已持久化到 `repo.key`
pub fn load_or_generate_repo_key(deve_dir: &Path) -> anyhow::Result<Option<RepoKey>> {
    let repo_key_path = deve_dir.join("repo.key");

    if repo_key_path.exists() {
        let bytes = std::fs::read(&repo_key_path)?;
        match RepoKey::from_bytes(&bytes) {
            Some(key) => {
                tracing::info!("Loaded RepoKey from {:?}", repo_key_path);
                Ok(Some(key))
            }
            None => {
                tracing::warn!("Invalid repo.key file, regenerating...");
                let key = RepoKey::generate();
                std::fs::write(&repo_key_path, key.to_bytes())?;
                Ok(Some(key))
            }
        }
    } else {
        // 生成新密钥并保存
        let key = RepoKey::generate();
        std::fs::write(&repo_key_path, key.to_bytes())?;
        tracing::info!("Generated and saved new RepoKey to {:?}", repo_key_path);
        Ok(Some(key))
    }
}
