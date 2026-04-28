//! 浏览器存储分层模块。
//! plan_ref:
//!   - 04_storage#browser-storage-layering
//!
//! 本模块落实 WebLightPeer 的四层信任模型：`localStorage` 仅保存 UI 偏好，
//! `IndexedDB` 保存 repo 级元数据与离线缓存，`WebCrypto` 持有不可导出的私钥材料。

pub mod identity;
mod js_bridge;
pub mod prefs;

use serde::{Deserialize, Serialize};

/// 浏览器存储能力探测结果。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCapabilities {
    pub webcrypto: bool,
    pub indexed_db: bool,
    pub local_storage: bool,
    pub ed25519: bool,
}

impl StorageCapabilities {
    /// 根据能力矩阵决定是否进入降级同步模式。
    pub fn degraded_mode(&self) -> Option<DegradedSyncMode> {
        (!self.webcrypto || !self.indexed_db || !self.ed25519).then(|| DegradedSyncMode {
            reason: format!(
                "WebCrypto={}, IndexedDB={}, Ed25519={}",
                self.webcrypto, self.indexed_db, self.ed25519
            ),
        })
    }
}

/// `DegradedSyncMode` 表示浏览器缺少持久 peer 身份能力时的只读降级态。
///
/// 在该模式下允许保留登录态、文档列表与快照拉取，但禁止 peer 注册、写入同步与 `SyncPush`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DegradedSyncMode {
    pub reason: String,
}

impl DegradedSyncMode {
    /// 返回顶部横幅文案，供 UI 呈现只读原因。
    pub fn banner_text(&self) -> String {
        format!(
            "浏览器持久存储不可用，已切换为只读同步模式：允许查看与拉取，禁止 Peer 注册、编辑提交与 SyncPush（{}）",
            self.reason
        )
    }
}

/// repo 级同步元数据，仅保存可恢复缓存，不承载私钥字节。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RepoMetadata {
    pub repo_id: String,
    pub vector_json: Option<String>,
    pub last_handshake_ms: Option<f64>,
}

/// 浏览器存储错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageError {
    Browser(String),
    Decode(String),
}

impl StorageError {
    /// 返回适合日志与 UI 的错误文本。
    pub fn message(&self) -> &str {
        match self {
            Self::Browser(msg) | Self::Decode(msg) => msg,
        }
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for StorageError {}

/// 浏览器存储结果类型别名。
pub type StorageResult<T> = Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::StorageCapabilities;

    #[test]
    fn storage_capabilities_allow_full_sync_when_identity_stack_exists() {
        let caps = StorageCapabilities {
            webcrypto: true,
            indexed_db: true,
            local_storage: true,
            ed25519: true,
        };

        assert_eq!(caps.degraded_mode(), None);
    }

    #[test]
    fn store_011_storage_capabilities_degrade_to_read_only_without_identity_stack() {
        // STORE-011: browser storage authority boundary.
        let caps = StorageCapabilities {
            webcrypto: true,
            indexed_db: false,
            local_storage: true,
            ed25519: true,
        };

        let mode = caps.degraded_mode().expect("missing IndexedDB degrades");
        assert!(mode.reason.contains("IndexedDB=false"));
        assert!(mode.banner_text().contains("只读同步模式"));
        assert!(mode.banner_text().contains("禁止 Peer 注册"));
        assert!(mode.banner_text().contains("SyncPush"));
    }
}
