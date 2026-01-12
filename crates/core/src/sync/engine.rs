//! # P2P 同步引擎 (P2P Sync Engine)
//!
//! **架构作用**:
//! 实现 Gossip 协议的核心同步逻辑。作为协调者，编排 Version Vector、Protocol 和 Repository。
//!
//! **核心功能清单**:
//! - `SyncEngine`: 协调者，管理 SyncLoop。
//! - `handshake`: 执行握手协议。
//! - `apply_remote_ops`: 应用操作。
//! - Manual 模式：通过 `PendingOpsBuffer` 管理暂存操作。
//!
//! **类型**: Core MUST (核心必选)

use std::sync::Arc;
use anyhow::Result;
use crate::models::PeerId;
use crate::sync::vector::VersionVector;
use crate::sync::protocol::{self, SyncRequest, SyncResponse, HandshakeResult};
use crate::sync::buffer::PendingOpsBuffer;
use crate::ledger::RepoManager;
use crate::config::SyncMode;

/// P2P 同步引擎
/// 
/// 管理本地与远端 Peer 的数据同步。
/// 协调 Version Vector 状态、差异计算与数据应用。
pub struct SyncEngine {
    /// 本地 Peer ID
    pub local_peer_id: PeerId,
    /// 仓库管理器
    repo: Arc<RepoManager>,
    /// 本地 Version Vector (追踪所有已知 Peer 的最新序列号)
    version_vector: VersionVector,
    /// 同步模式 (Auto/Manual)
    sync_mode: SyncMode,
    /// 待合并的操作缓冲区 (Manual 模式下使用)
    pending_ops: PendingOpsBuffer,
}

impl SyncEngine {
    /// 创建新的同步引擎
    pub fn new(local_peer_id: PeerId, repo: Arc<RepoManager>, sync_mode: SyncMode) -> Self {
        Self {
            local_peer_id,
            repo,
            version_vector: VersionVector::new(),
            sync_mode,
            pending_ops: PendingOpsBuffer::new(),
        }
    }

    /// 获取当前同步模式
    pub fn sync_mode(&self) -> SyncMode {
        self.sync_mode
    }

    /// 设置同步模式
    pub fn set_sync_mode(&mut self, mode: SyncMode) {
        self.sync_mode = mode;
    }

    /// 检查是否有待合并的操作 (Manual 模式)
    pub fn has_pending_ops(&self) -> bool {
        !self.pending_ops.is_empty()
    }

    /// 获取待合并操作的数量
    pub fn pending_ops_count(&self) -> usize {
        self.pending_ops.count()
    }

    /// 获取当前 Version Vector 的引用
    pub fn version_vector(&self) -> &VersionVector {
        &self.version_vector
    }

    /// 获取当前 Version Vector 的可变引用
    pub fn version_vector_mut(&mut self) -> &mut VersionVector {
        &mut self.version_vector
    }

    /// 更新本地序列号
    pub fn update_local_seq(&mut self, seq: u64) {
        self.version_vector.update(self.local_peer_id.clone(), seq);
    }

    /// 更新远端 Peer 的序列号
    pub fn update_remote_seq(&mut self, peer_id: PeerId, seq: u64) {
        self.version_vector.update(peer_id, seq);
    }

    /// 计算与远端 Peer 的差异
    /// 代理给 `protocol::compute_diff_requests`
    pub fn compute_diff(&self, remote_vector: &VersionVector) -> (Vec<SyncRequest>, Vec<SyncRequest>) {
        protocol::compute_diff_requests(&self.version_vector, remote_vector)
    }

    /// 从本地仓库获取指定范围的操作 (用于发送给远端)
    /// 
    /// 这会从 Local Repo 获取本地产生的操作，
    /// 或从 Shadow Repo 获取之前从其他 Peer 接收的操作。
    pub fn get_ops_for_sync(&self, request: &SyncRequest) -> Result<SyncResponse> {
        let ops = if request.peer_id == self.local_peer_id {
            // 请求的是本地数据 - 从 Local Repo 获取
            self.repo.get_local_ops_in_range(request.range.0, request.range.1)?
        } else {
            // 请求的是远端数据 - 从 Shadow Repo 获取
            self.repo.get_shadow_ops_in_range(&request.peer_id, request.range.0, request.range.1)?
        };

        Ok(SyncResponse {
            peer_id: request.peer_id.clone(),
            ops,
        })
    }

    /// 应用从远端接收的操作
    /// 
    /// 将操作写入对应的 Shadow Repo，并更新 Version Vector。
    pub fn apply_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        let mut max_seq = 0u64;

        for (seq, entry) in response.ops {
            self.repo.append_remote_op(&response.peer_id, &entry)?;
            max_seq = max_seq.max(seq);
        }

        // 更新 Version Vector
        if max_seq > 0 {
            self.version_vector.update(response.peer_id, max_seq);
        }

        Ok(max_seq)
    }

    /// 执行完整的握手流程
    pub fn handshake(&mut self, _remote_peer_id: PeerId, remote_vector: VersionVector) -> HandshakeResult {
        let (to_send, to_request) = self.compute_diff(&remote_vector);
        
        HandshakeResult {
            to_send,
            to_request,
            auto_apply: self.sync_mode == SyncMode::Auto,
        }
    }

    /// 暂存从远端接收的操作 (Manual 模式)
    pub fn buffer_remote_ops(&mut self, response: SyncResponse) {
        self.pending_ops.push(response);
    }

    /// 合并所有待处理的操作 (Manual 模式显式触发)
    pub fn merge_pending(&mut self) -> Result<u64> {
        let mut total = 0u64;
        let pending = self.pending_ops.take_all();
        
        for response in pending {
            let count = self.apply_remote_ops(response)?;
            total += count;
        }
        
        Ok(total)
    }

    /// 清空待处理的操作 (丢弃不合并)
    pub fn clear_pending(&mut self) {
        self.pending_ops.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_engine(peer_id: &str, mode: SyncMode) -> (TempDir, SyncEngine) {
        let tmp_dir = TempDir::new().unwrap();
        let ledger_dir = tmp_dir.path().join("ledger");
        let repo = Arc::new(RepoManager::init(&ledger_dir, 10).unwrap());
        let engine = SyncEngine::new(PeerId::new(peer_id), repo, mode);
        (tmp_dir, engine)
    }

    #[test]
    fn test_compute_diff_basic() {
        let (_tmp, mut engine) = setup_engine("local", SyncMode::Auto);

        // Local: A=10, B=5
        engine.update_local_seq(10);
        engine.version_vector_mut().update(PeerId::new("peer_b"), 5);

        // Remote: A=5, B=10
        let mut remote_vec = VersionVector::new();
        remote_vec.update(PeerId::new("local"), 5);
        remote_vec.update(PeerId::new("peer_b"), 10);

        let (to_send, to_request) = engine.compute_diff(&remote_vec);

        // We should send: local 6..11 (our local data)
        assert!(to_send.iter().any(|r| r.peer_id.as_str() == "local" && r.range == (6, 11)));

        // We should request: peer_b 6..11 (their data we're missing)
        assert!(to_request.iter().any(|r| r.peer_id.as_str() == "peer_b" && r.range == (6, 11)));
    }

    #[test]
    fn test_sync_mode_auto_handshake() {
        let (_tmp, mut engine) = setup_engine("local", SyncMode::Auto);
        engine.update_local_seq(5);

        let mut remote_vec = VersionVector::new();
        remote_vec.update(PeerId::new("local"), 3);

        let result = engine.handshake(PeerId::new("remote"), remote_vec);
        
        assert!(result.auto_apply, "Auto mode should have auto_apply=true");
        assert!(!result.to_send.is_empty());
    }

    #[test]
    fn test_sync_mode_manual_handshake() {
        let (_tmp, mut engine) = setup_engine("local", SyncMode::Manual);
        engine.update_local_seq(5);

        let mut remote_vec = VersionVector::new();
        remote_vec.update(PeerId::new("local"), 3);

        let result = engine.handshake(PeerId::new("remote"), remote_vec);
        
        assert!(!result.auto_apply, "Manual mode should have auto_apply=false");
    }

    #[test]
    fn test_manual_mode_pending_ops() {
        let (_tmp, mut engine) = setup_engine("local", SyncMode::Manual);
        
        assert!(!engine.has_pending_ops());
        assert_eq!(engine.pending_ops_count(), 0);

        // Buffer some ops
        let response = SyncResponse {
            peer_id: PeerId::new("remote"),
            ops: vec![], // Empty for testing
        };
        engine.buffer_remote_ops(response);

        assert!(engine.has_pending_ops());

        // Clear pending
        engine.clear_pending();
        assert!(!engine.has_pending_ops());
    }
}
