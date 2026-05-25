//! plan_ref:
//!   - 03_storage#browser-storage-layering
//!   - 04_repository#repo-scope-runtime
//!
use crate::storage::DegradedSyncMode;
use crate::storage::identity::{
    StoredPeerIdentity, load_or_create_identity, load_repo_metadata, probe_capabilities,
    touch_offline_cache,
};
use deve_core::models::VersionVector;

pub(super) struct StorageBootstrap {
    pub(super) identity: StoredPeerIdentity,
    pub(super) vector: VersionVector,
}

pub(super) async fn bootstrap_repo_storage(
    repo_id: &str,
) -> Result<StorageBootstrap, DegradedSyncMode> {
    match probe_capabilities().await {
        Ok(capabilities) => {
            if let Some(mode) = capabilities.degraded_mode() {
                return Err(mode);
            }
        }
        Err(err) => {
            return Err(DegradedSyncMode {
                reason: format!("能力探测失败: {}", err),
            });
        }
    }

    let identity = load_or_create_identity(repo_id)
        .await
        .map_err(|err| DegradedSyncMode {
            reason: format!("无法恢复浏览器身份: {}", err),
        })?;

    let metadata = match load_repo_metadata(repo_id).await {
        Ok(metadata) => metadata,
        Err(err) => {
            leptos::logging::warn!("加载 repo 元数据失败 {}: {}", repo_id, err);
            Default::default()
        }
    };

    let vector = match metadata.vector_json.as_deref() {
        Some(json) => match serde_json::from_str(json) {
            Ok(vector) => vector,
            Err(err) => {
                leptos::logging::warn!("解析 repo 向量失败 {}: {}", repo_id, err);
                VersionVector::new()
            }
        },
        None => VersionVector::new(),
    };

    if let Err(err) = touch_offline_cache(repo_id, "bootstrap").await {
        leptos::logging::warn!("离线缓存触点更新失败: {}", err);
    }

    Ok(StorageBootstrap { identity, vector })
}
