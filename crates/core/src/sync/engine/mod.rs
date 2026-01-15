use crate::models::PeerId;
use crate::sync::vector::VersionVector;
use crate::sync::protocol::{SyncRequest, SyncResponse};
use anyhow::Result;

pub mod handshake;
pub mod transfer;
pub mod manual;

/// P2P 同步引擎
/// 
/// **功能**:
/// 管理本地与远端 Peer 的数据同步。
/// 协调 Version Vector 状态、差异计算与数据应用。
pub struct SyncEngine {
    pub local_peer_id: PeerId,
    pub repo: std::sync::Arc<crate::ledger::RepoManager>,
    pub version_vector: VersionVector,
    pub sync_mode: crate::config::SyncMode,
    pub pending_ops: crate::sync::buffer::PendingOpsBuffer,
    pub repo_key: Option<crate::security::RepoKey>, // Encryption Key
}

impl SyncEngine {
    pub fn new(
        local_peer_id: PeerId,
        repo: std::sync::Arc<crate::ledger::RepoManager>,
        sync_mode: crate::config::SyncMode,
        repo_key: Option<crate::security::RepoKey>,
    ) -> Self {
        Self {
            local_peer_id,
            repo,
            version_vector: VersionVector::new(),
            sync_mode,
            pending_ops: crate::sync::buffer::PendingOpsBuffer::new(),
            repo_key,
        }
    }

    pub fn sync_mode(&self) -> crate::config::SyncMode {
        self.sync_mode
    }

    pub fn set_sync_mode(&mut self, mode: crate::config::SyncMode) {
        self.sync_mode = mode;
    }

    pub fn version_vector(&self) -> &VersionVector {
        &self.version_vector
    }

    pub fn version_vector_mut(&mut self) -> &mut VersionVector {
        &mut self.version_vector
    }

    pub fn update_local_seq(&mut self, seq: u64) {
        self.version_vector.update(self.local_peer_id.clone(), seq);
    }

    pub fn update_remote_seq(&mut self, peer_id: PeerId, seq: u64) {
        self.version_vector.update(peer_id, seq);
    }
}
