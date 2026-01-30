use super::SyncEngine;
use crate::models::{LedgerEntry, Op};
use crate::sync::protocol::{SyncResponse, SyncSnapshotRequest};
use crate::sync::rebuild;
use anyhow::Result;

impl SyncEngine {
    /// 获取快照数据 (用于全量同步)。
    ///
    /// Invariants:
    /// - 快照的 `seq` 反映源端对该文档的最新已知序列号。
    /// - 快照内容由“最新快照 + 增量操作”重建得出。
    pub fn get_snapshot_for_sync(&self, request: &SyncSnapshotRequest) -> Result<SyncResponse> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;

        // 当前仍以主仓库为来源；request.repo_id 仅用于回包标识。
        let repo_name = self.repo.local_repo_name();
        let docs = self.repo.list_local_docs(Some(repo_name))?;

        let mut ops = Vec::new();
        for (doc_id, _) in docs {
            let rebuilt = rebuild::rebuild_local_doc(&self.repo, doc_id)?;
            if rebuilt.content.is_empty() {
                continue;
            }

            let latest_seq = rebuilt.max_seq;
            let entry = LedgerEntry {
                doc_id,
                op: Op::Insert {
                    pos: 0,
                    content: rebuilt.content.into(),
                },
                timestamp: chrono::Utc::now().timestamp_millis(),
                peer_id: self.local_peer_id.clone(),
                seq: latest_seq,
            };

            ops.push(repo_key.encrypt(&entry, latest_seq)?);
        }

        Ok(SyncResponse {
            peer_id: self.local_peer_id.clone(),
            repo_id: request.repo_id,
            ops,
        })
    }
}
