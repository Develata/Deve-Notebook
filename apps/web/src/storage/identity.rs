//! 浏览器 peer identity 持久化。
//!
//! 私钥材料始终停留在 `WebCrypto` 的不可导出 `CryptoKey` 中；Rust 侧仅消费公钥与 repo 级元数据。

use super::{RepoMetadata, StorageCapabilities, StorageError, StorageResult, js_bridge};
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

/// 浏览器持久化后的 peer identity 元数据。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPeerIdentity {
    pub repo_id: String,
    pub peer_id: String,
    pub public_key: Vec<u8>,
    pub created_at: u64,
}

/// 探测 WebCrypto、IndexedDB 与 `localStorage` 能力。
pub async fn probe_capabilities() -> StorageResult<StorageCapabilities> {
    resolve_json(js_bridge::probe_storage_capabilities()).await
}

/// 载入或首次生成 repo 级浏览器 peer identity。
pub async fn load_or_create_identity(repo_id: &str) -> StorageResult<StoredPeerIdentity> {
    resolve_json(js_bridge::load_or_create_identity(repo_id)).await
}

/// 读取 repo 级版本向量与握手缓存元数据。
pub async fn load_repo_metadata(repo_id: &str) -> StorageResult<RepoMetadata> {
    resolve_json(js_bridge::load_repo_meta(repo_id)).await
}

/// 保存 repo 级版本向量缓存。
pub async fn save_repo_vector(repo_id: &str, vector_json: &str) -> StorageResult<()> {
    merge_repo_meta(
        repo_id,
        RepoMetadata {
            repo_id: repo_id.into(),
            vector_json: Some(vector_json.into()),
            last_handshake_ms: None,
        },
    )
    .await
}

/// 记录最近一次成功握手时间。
pub async fn note_handshake(repo_id: &str) -> StorageResult<()> {
    merge_repo_meta(
        repo_id,
        RepoMetadata {
            repo_id: repo_id.into(),
            vector_json: None,
            last_handshake_ms: Some(js_sys::Date::now()),
        },
    )
    .await
}

/// 标记离线缓存桶已被访问，用于 repo 级缓存隔离。
pub async fn touch_offline_cache(repo_id: &str, cache_key: &str) -> StorageResult<()> {
    JsFuture::from(js_bridge::touch_offline_cache(repo_id, cache_key))
        .await
        .map_err(js_error)?;
    Ok(())
}

/// 使用 `WebCrypto` 私钥为握手消息签名。
pub async fn sign_sync_hello(
    identity: &StoredPeerIdentity,
    message: &[u8],
) -> StorageResult<Vec<u8>> {
    let sig = JsFuture::from(js_bridge::sign_peer_message(
        &identity.repo_id,
        &Uint8Array::from(message),
    ))
    .await
    .map_err(js_error)?;
    Ok(Uint8Array::new(&sig).to_vec())
}

async fn merge_repo_meta(repo_id: &str, patch: RepoMetadata) -> StorageResult<()> {
    let json = serde_json::to_string(&patch).map_err(|e| StorageError::Decode(e.to_string()))?;
    JsFuture::from(js_bridge::merge_repo_meta(repo_id, &json))
        .await
        .map_err(js_error)?;
    Ok(())
}

async fn resolve_json<T: DeserializeOwned>(promise: js_sys::Promise) -> StorageResult<T> {
    let raw = JsFuture::from(promise).await.map_err(js_error)?;
    let text = raw
        .as_string()
        .ok_or_else(|| StorageError::Decode("JS bridge did not return JSON string".into()))?;
    serde_json::from_str(&text).map_err(|e| StorageError::Decode(e.to_string()))
}

fn js_error(value: JsValue) -> StorageError {
    StorageError::Browser(value.as_string().unwrap_or_else(|| format!("{value:?}")))
}
