//! 浏览器存储分层模块。
//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!
//! 本模块落实 WebLightPeer 的四层信任模型：`localStorage` 仅保存 UI 偏好，
//! `IndexedDB` 保存 repo 级元数据与离线缓存，`WebCrypto` 持有不可导出的私钥材料。

mod capability;
pub mod identity;
mod js_bridge;
pub mod prefs;

pub use capability::{BrowserIdentityBlocker, DegradedSyncMode, StorageCapabilities};
use serde::{Deserialize, Serialize};

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
