// crates\core\src\sync\engine
//! # P2P 同步引擎 (Sync Engine)
//!
//! **架构作用**:
//! 管理本地与远端 Peer 的数据同步，协调 Version Vector 状态、差异计算与数据应用。
//!
//! ## 状态机模型 (State Machine Model)
//!
//! ```text
//! ┌─────────────┐
//! │    Idle     │ ← 初始状态
//! └──────┬──────┘
//!        │ SyncHello (收到握手)
//!        ▼
//! ┌─────────────┐
//! │  Handshake  │ ← 交换 Version Vector
//! └──────┬──────┘
//!        │ VV Diff 计算完成
//!        ▼
//! ┌─────────────┐
//! │  Transfer   │ ← 数据传输 (Pull/Push)
//! └──────┬──────┘
//!        │ 所有 Ops 已同步
//!        ▼
//! ┌─────────────┐
//! │   Synced    │ ← 同步完成
//! └─────────────┘
//! ```
//!
//! ## 同步模式 (Sync Mode)
//!
//! - **Auto**: 收到数据后立即应用到本地存储。
//! - **Manual**: 数据暂存于 `PendingOpsBuffer`，等待用户确认后再应用。
//!
//! ## 数学不变量 (Mathematical Invariants)
//!
//! 1. **Version Vector 单调性**:
//!    ```text
//!    ∀ t1 < t2 : engine.version_vector(t1) ⊆ engine.version_vector(t2)
//!    ```
//!    其中 `⊆` 表示点态小于等于 (pointwise ≤)。
//!
//! 2. **Pull 完整性**:
//!    ```text
//!    ∀ (peer, start, end) ∈ calculate_pull_ranges(remote_vv):
//!      local_vv.get(peer) < end ∧ remote_vv.get(peer) ≥ end
//!    ```
//!
//! 3. **Push 有效性**:
//!    ```text
//!    ∀ (peer, start, end) ∈ calculate_push_ranges(remote_vv):
//!      local_vv.get(peer) ≥ end ∧ remote_vv.get(peer) < start
//!    ```

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
///
/// ## 不变量 (Invariants)
///
/// - `version_vector` 中的所有序列号单调递增。
/// - `pending_ops` 仅在 `Manual` 模式下累积未应用的操作。
/// - `repo_key` 用于加密/解密传输的 Ops (可选)。
pub struct SyncEngine {
    pub local_peer_id: PeerId,
    pub repo: std::sync::Arc<crate::ledger::RepoManager>,
    pub version_vector: VersionVector,
    pub sync_mode: crate::config::SyncMode,
    pub pending_ops: crate::sync::buffer::PendingOpsBuffer,
    pub repo_key: Option<crate::security::RepoKey>, // Encryption Key
}

impl SyncEngine {
    /// 创建新的同步引擎实例。
    ///
    /// ## 后置条件 (Post-conditions)
    /// - `self.version_vector` 为空向量。
    /// - `self.pending_ops` 为空缓冲区。
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

    /// 更新本地节点的序列号。
    ///
    /// ## 前置条件 (Pre-conditions)
    /// - `seq > 0`
    ///
    /// ## 后置条件 (Post-conditions)
    /// - `self.version_vector.get(&self.local_peer_id) >= seq`
    pub fn update_local_seq(&mut self, seq: u64) {
        self.version_vector.update(self.local_peer_id.clone(), seq);
    }

    /// 更新远端节点的序列号。
    ///
    /// ## 前置条件 (Pre-conditions)
    /// - `seq > 0`
    ///
    /// ## 后置条件 (Post-conditions)
    /// - `self.version_vector.get(&peer_id) >= seq`
    pub fn update_remote_seq(&mut self, peer_id: PeerId, seq: u64) {
        self.version_vector.update(peer_id, seq);
    }

    /// 计算需要从远端拉取的操作范围。
    ///
    /// ## 数学定义
    /// ```text
    /// pull_ranges = { (p, local[p]+1, remote[p]) | remote[p] > local[p] }
    /// ```
    ///
    /// ## 返回值
    /// `Vec<(PeerId, start_seq, end_seq)>` - 需要从远端拉取的序列号范围 (左闭右闭)。
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

    /// 计算可以推送给远端的操作范围。
    ///
    /// ## 数学定义
    /// ```text
    /// push_ranges = { (p, remote[p]+1, local[p]) | local[p] > remote[p] }
    /// ```
    ///
    /// ## 返回值
    /// `Vec<(PeerId, start_seq, end_seq)>` - 可以推送给远端的序列号范围 (左闭右闭)。
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
