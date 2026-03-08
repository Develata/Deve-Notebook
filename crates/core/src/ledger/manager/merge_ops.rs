// crates/core/src/ledger/manager/merge_ops.rs
//! # P2P 合并操作
//!
//! 实现 `RepoManager` 的 `merge_peer` 方法。

use crate::ledger::RepoManager;
use crate::ledger::merge::{MergeEngine, MergeResult};
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
        self.merge_peer_in_local_repo(self.local_repo_name(), peer_id, repo_id, doc_id)
    }

    /// 合并指定 Peer 的分支到目标本地仓库。
    ///
    /// Invariants:
    /// - `repo_name` 与 `repo_id` 必须描述同一个本地 repo 作用域。
    /// - base/local/remote 三方内容都必须从同一个 doc_id 的确定性日志重建。
    pub fn merge_peer_in_local_repo(
        &self,
        repo_name: &str,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
    ) -> Result<MergeResult> {
        let local_ops = self.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let remote_ops = match self.get_ops(&RepoType::Remote(peer_id.clone(), *repo_id), doc_id) {
            Ok(ops) => ops,
            Err(e) => {
                tracing::warn!(
                    "merge_peer: 无法读取远端 ops (peer={}, repo={}): {:?}",
                    peer_id,
                    repo_id,
                    e
                );
                // 远端无数据时视为空集 — 合并结果等于本地内容
                Vec::new()
            }
        };
        Ok(merge_ops(doc_id, local_ops, remote_ops))
    }
}

fn merge_ops(
    doc_id: DocId,
    local_ops: Vec<(u64, LedgerEntry)>,
    remote_ops: Vec<(u64, LedgerEntry)>,
) -> MergeResult {
    let local_vv = build_version_vector(&local_ops);
    let remote_vv = build_version_vector(&remote_ops);
    let lca_vv = MergeEngine::find_lca(&local_vv, &remote_vv);

    let all_local_entries: Vec<LedgerEntry> = local_ops.iter().map(|(_, e)| e.clone()).collect();
    let all_remote_entries: Vec<LedgerEntry> = remote_ops.iter().map(|(_, e)| e.clone()).collect();
    let pooled_entries = dedup_entries(local_ops, remote_ops);

    let base_content = MergeEngine::reconstruct_state_at(doc_id, &pooled_entries, &lca_vv);
    let local_content = MergeEngine::reconstruct_state_at(doc_id, &all_local_entries, &local_vv);
    let remote_content = MergeEngine::reconstruct_state_at(doc_id, &all_remote_entries, &remote_vv);

    MergeEngine::merge_commits(&base_content, &local_content, &remote_content)
}

fn build_version_vector(entries: &[(u64, LedgerEntry)]) -> VersionVector {
    let mut vector = VersionVector::new();
    for (_, entry) in entries {
        vector.update(entry.peer_id.clone(), entry.seq);
    }
    vector
}

fn dedup_entries(
    local_ops: Vec<(u64, LedgerEntry)>,
    remote_ops: Vec<(u64, LedgerEntry)>,
) -> Vec<LedgerEntry> {
    let mut pooled_entries = Vec::with_capacity(local_ops.len() + remote_ops.len());
    pooled_entries.extend(local_ops.into_iter().map(|(_, entry)| entry));
    pooled_entries.extend(remote_ops.into_iter().map(|(_, entry)| entry));
    pooled_entries.sort_by(|a, b| a.peer_id.cmp(&b.peer_id).then_with(|| a.seq.cmp(&b.seq)));
    pooled_entries.dedup_by(|a, b| a.peer_id == b.peer_id && a.seq == b.seq);
    pooled_entries
}
