//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::sync::protocol::{SyncRequest, SyncResponse};
use anyhow::Result;

mod apply;
mod snapshot;

pub(super) use apply::{decrypt_remote_ops, validate_full_fact_replay, validate_incremental_range};

impl SyncEngine {
    /// 从本地仓库获取指定范围的操作 (用于发送给远端)。
    ///
    /// **安全**: 使用 `RepoKey` 对 LedgerEntry 进行加密 (Envelope Pattern)。
    pub fn get_ops_for_sync(&self, request: &SyncRequest) -> Result<SyncResponse> {
        let raw_ops = if request.peer_id == self.local_peer_id {
            self.repo.get_local_ops_in_range(
                &request.repo_id,
                &request.peer_id,
                request.range.0,
                request.range.1,
            )?
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
        for (_storage_seq, entry) in raw_ops {
            encrypted_ops.push(repo_key.encrypt(&entry, entry.peer_seq)?);
        }

        Ok(SyncResponse {
            peer_id: request.peer_id.clone(),
            repo_id: request.repo_id,
            range: Some(request.range),
            waterline: crate::models::PeerFactSeq::ZERO,
            ops: encrypted_ops,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SyncMode;
    use crate::ledger::RepoManager;
    use crate::models::{DocId, LedgerEntry, Op};
    use crate::security::RepoKey;
    use std::sync::Arc;

    #[test]
    fn get_ops_for_sync_envelope_uses_payload_peer_seq_not_storage_seq() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let repo = Arc::new(RepoManager::init(
            dir.path().join("ledger"),
            8,
            Some("default"),
            Some("urn:default"),
        )?);
        let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
        let peer_id = repo.local_peer_id().clone();
        let repo_key = RepoKey::generate();
        let engine = SyncEngine::new(
            peer_id.clone(),
            repo.clone(),
            SyncMode::Auto,
            Some(repo_key.clone()),
        );
        let doc_id = DocId::new();
        let peer_for_entry = peer_id.clone();
        let (_storage_seq, peer_seq) = repo.append_generated_op_in_local_repo(
            "default",
            doc_id,
            peer_id.clone(),
            move |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    Op::Insert {
                        pos: 0,
                        content: "hello".into(),
                    },
                    1,
                    peer_for_entry.clone(),
                    seq,
                    None,
                    None,
                )
            },
        )?;
        let response = engine.get_ops_for_sync(&SyncRequest {
            peer_id,
            repo_id,
            range: (peer_seq.into(), peer_seq.into()),
        })?;

        assert_eq!(response.ops.len(), 1);
        assert_eq!(response.ops[0].peer_seq, peer_seq);
        assert_eq!(repo_key.decrypt(&response.ops[0])?.peer_seq.get(), peer_seq);
        Ok(())
    }
}
