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
    /// - 快照内容由"最新快照 + 增量操作"重建得出。
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
        let docs = self.repo.list_local_docs(Some(&repo_name))?;

        let mut ops = Vec::new();
        for (doc_id, _) in docs {
            let rebuilt = rebuild::rebuild_local_doc_in_repo(&self.repo, &repo_name, doc_id)?;
            if rebuilt.content.is_empty() {
                continue;
            }

            let latest_seq = rebuilt.max_seq;
            let entry = LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: rebuilt.content.into(),
                },
                chrono::Utc::now().timestamp_millis(),
                self.local_peer_id.clone(),
                latest_seq,
                None,
                None,
            );

            ops.push(repo_key.encrypt(&entry, latest_seq)?);
        }

        Ok(SyncResponse {
            peer_id: self.local_peer_id.clone(),
            repo_id: request.repo_id,
            ops,
        })
    }
}
