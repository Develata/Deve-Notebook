use super::SyncEngine;
use crate::ledger::range;
use crate::sync::protocol::{SyncResponse, SyncSnapshotRequest};
use anyhow::Result;

impl SyncEngine {
    /// 获取快照数据 (用于全量同步)。
    ///
    /// Invariants:
    /// - 快照必须携带完整 Ledger Facts，而不是重建后的伪内容 op。
    /// - 快照序列号必须保留原始全局顺序，供远端整库重放。
    /// - 必须按请求的 `repo_id` 获取数据，不能默认使用本地主仓库。
    pub fn get_snapshot_for_sync(&self, request: &SyncSnapshotRequest) -> Result<SyncResponse> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;
        let repo_name = self
            .repo
            .find_local_repo_name_by_id(request.repo_id)?
            .ok_or_else(|| anyhow::anyhow!("Local repo not found for UUID {}", request.repo_id))?;
        let max_seq = self
            .repo
            .run_on_local_repo(&repo_name, range::get_max_seq)?;
        let raw_ops = if max_seq == 0 {
            Vec::new()
        } else {
            self.repo
                .get_local_ops_in_range(&request.repo_id, 1, max_seq.saturating_add(1))?
        };
        let mut ops = Vec::with_capacity(raw_ops.len());
        for (seq, entry) in raw_ops {
            ops.push(repo_key.encrypt(&entry, seq)?);
        }

        Ok(SyncResponse {
            peer_id: self.local_peer_id.clone(),
            repo_id: request.repo_id,
            ops,
        })
    }
}
