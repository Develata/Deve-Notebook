use super::SyncEngine;
use crate::sync::protocol::{SyncRequest, SyncResponse};
use anyhow::Result;

mod apply;
mod snapshot;

impl SyncEngine {
    /// 从本地仓库获取指定范围的操作 (用于发送给远端)。
    ///
    /// **安全**: 使用 `RepoKey` 对 LedgerEntry 进行加密 (Envelope Pattern)。
    pub fn get_ops_for_sync(&self, request: &SyncRequest) -> Result<SyncResponse> {
        let raw_ops = if request.peer_id == self.local_peer_id {
            self.repo
                .get_local_ops_in_range(&request.repo_id, request.range.0, request.range.1)?
        } else {
            self.repo.get_shadow_ops_in_range(
                &request.peer_id,
                &request.repo_id,
                request.range.0,
                request.range.1,
            )?
        };

        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot encrypt ops"))?;

        let mut encrypted_ops = Vec::with_capacity(raw_ops.len());
        for (seq, entry) in raw_ops {
            encrypted_ops.push(repo_key.encrypt(&entry, seq)?);
        }

        Ok(SyncResponse {
            peer_id: request.peer_id.clone(),
            repo_id: request.repo_id,
            ops: encrypted_ops,
        })
    }
}

