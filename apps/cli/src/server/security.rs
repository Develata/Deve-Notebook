// apps/cli/src/server/security.rs
//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 05_network#server-ws-runtime
//!   - 09_auth#key-and-file-permissions
//!
//! # 安全密钥管理模块
//!
//! 管理宿主 Identity Key 的加载与生成。
//!
//! ## 不变量 (Invariants)
//! - Identity Key 必须始终存在 (首次启动时自动生成)
//! - Repo Key 可选，但一旦生成必须保持一致性

use anyhow::{Context, Result, bail};
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
    enforce_owner_only_identity_key(&host_key_dir.join("identity.key"))?;
    tracing::info!(
        "IdentityKey ready at {:?}",
        host_key_dir.join("identity.key")
    );
    Ok(kp)
}

#[cfg(unix)]
fn enforce_owner_only_identity_key(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to read identity key metadata: {:?}", path))?;
    if !metadata.is_file() {
        bail!("Identity key path is not a file: {:?}", path);
    }

    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o600 {
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(path, permissions).with_context(|| {
            format!(
                "Failed to set owner-only identity key permissions: {:?}",
                path
            )
        })?;
        tracing::warn!(
            path = ?path,
            old_mode = format_args!("{mode:o}"),
            new_mode = "600",
            "Corrected identity key permissions to owner-only"
        );
    }
    Ok(())
}

#[cfg(not(unix))]
fn enforce_owner_only_identity_key(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to read identity key metadata: {:?}", path))?;
    if !metadata.is_file() {
        bail!("Identity key path is not a file: {:?}", path);
    }
    tracing::warn!(
        path = ?path,
        "Owner-only identity key permission enforcement is not portable on this platform"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::enforce_owner_only_identity_key;

    #[cfg(unix)]
    #[test]
    fn identity_key_permissions_are_corrected_to_owner_only() -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir()?;
        let path = dir.path().join("identity.key");
        std::fs::write(&path, "test-key")?;
        let mut permissions = std::fs::metadata(&path)?.permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(&path, permissions)?;

        enforce_owner_only_identity_key(&path)?;

        let mode = std::fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        Ok(())
    }

    #[test]
    fn identity_key_permissions_fail_closed_for_non_file() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let err = enforce_owner_only_identity_key(dir.path())
            .expect_err("directory identity key path must fail closed");

        assert!(err.to_string().contains("not a file"));
        Ok(())
    }
}
