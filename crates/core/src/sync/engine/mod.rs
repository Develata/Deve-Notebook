// crates\core\src\sync\engine
use crate::models::PeerId;
use crate::sync::vector::VersionVector;

pub mod handshake;
pub mod manual;
pub mod transfer;

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

    /// Calculate the difference between local VV and remote VV to determine missing ops.
    /// Returns a list of (start_seq, end_seq) ranges that we need to pull from remote.
    pub fn calculate_pull_ranges(&self, remote_vv: &VersionVector) -> Vec<(PeerId, u64, u64)> {
        let mut ranges = Vec::new();
        for (peer, remote_seq) in remote_vv.iter() {
            let local_seq = self.version_vector.get(peer);
            if *remote_seq > local_seq {
                ranges.push((peer.clone(), local_seq + 1, *remote_seq));
            }
        }
        ranges
    }

    /// Calculate the difference to determine ops we can push to remote.
    /// Returns a list of (start_seq, end_seq) ranges that we can push.
    pub fn calculate_push_ranges(&self, remote_vv: &VersionVector) -> Vec<(PeerId, u64, u64)> {
        let mut ranges = Vec::new();
        for (peer, local_seq) in self.version_vector.iter() {
            let remote_seq = remote_vv.get(peer);
            if *local_seq > remote_seq {
                ranges.push((peer.clone(), remote_seq + 1, *local_seq));
            }
        }
        ranges
    }
}
