// crates/core/src/ledger/manager/merge_ops.rs
//! # P2P 合并操作
//!
//! 实现 `RepoManager` 的 `merge_peer` 方法。

use crate::ledger::merge::{MergeEngine, MergeResult};
use crate::ledger::RepoManager;
use crate::models::{DocId, LedgerEntry, PeerId, RepoId, RepoType, VersionVector};
use anyhow::Result;

impl RepoManager {
    /// 合并指定 Peer 的分支到本地
    ///
    /// **流程**:
    /// 1. 获取本地和远端操作
    /// 2. 计算各自的 Version Vector
    /// 3. 找到 LCA (Lowest Common Ancestor)
    /// 4. 重建 base/local/remote 内容
    /// 5. 执行三方合并
    pub fn merge_peer(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
    ) -> Result<MergeResult> {
        // 1. 获取操作
        let local_ops = self.get_local_ops(doc_id)?;
        let remote_ops = match self.get_ops(&RepoType::Remote(peer_id.clone(), *repo_id), doc_id) {
            Ok(ops) => ops,
            Err(_) => return Ok(MergeResult::Success(String::new())),
        };

        // 2. 计算 Version Vectors
        let mut local_vv = VersionVector::new();
        for (_, entry) in &local_ops {
            local_vv.update(entry.peer_id.clone(), entry.seq);
        }

        let mut remote_vv = VersionVector::new();
        for (_, entry) in &remote_ops {
            remote_vv.update(entry.peer_id.clone(), entry.seq);
        }

        // 3. 计算 LCA
        let lca_vv = MergeEngine::find_lca(&local_vv, &remote_vv);

        // 4. 重建内容
        let all_local_entries: Vec<LedgerEntry> =
            local_ops.iter().map(|(_, e)| e.clone()).collect();

        // 合并操作池 (去重优化)
        let mut pooled_entries: Vec<LedgerEntry> =
            Vec::with_capacity(local_ops.len() + remote_ops.len());
        pooled_entries.extend(local_ops.iter().map(|(_, e)| e.clone()));
        pooled_entries.extend(remote_ops.iter().map(|(_, e)| e.clone()));

        // 按 (PeerId, Seq) 排序以对齐重复项
        pooled_entries.sort_by(|a, b| a.peer_id.cmp(&b.peer_id).then_with(|| a.seq.cmp(&b.seq)));

        // 去重: 移除连续的 (PeerId, Seq) 相同的条目
        pooled_entries.dedup_by(|a, b| a.peer_id == b.peer_id && a.seq == b.seq);

        let base_content = MergeEngine::reconstruct_state_at(doc_id, &pooled_entries, &lca_vv);
        let local_content =
            MergeEngine::reconstruct_state_at(doc_id, &all_local_entries, &local_vv);

        let all_remote_entries: Vec<LedgerEntry> =
            remote_ops.iter().map(|(_, e)| e.clone()).collect();
        let remote_content =
            MergeEngine::reconstruct_state_at(doc_id, &all_remote_entries, &remote_vv);

        // 5. 执行三方合并
        Ok(MergeEngine::merge_commits(
            &base_content,
            &local_content,
            &remote_content,
        ))
    }
}
