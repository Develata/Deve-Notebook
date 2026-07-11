//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::protocol::MAX_SYNC_FACT_BYTES_PER_PAYLOAD;
use crate::sync::protocol::{SyncResponse, SyncSnapshotRequest};
use anyhow::Result;

impl SyncEngine {
    /// 获取快照数据 (用于全量同步)。
    ///
    /// Invariants:
    /// - 快照必须携带完整 Ledger Facts，而不是重建后的伪内容 op。
    /// - replay 必须按 source peer 的 `PeerFactSeq` 1..=waterline 严格连续。
    /// - 必须按请求的 `repo_id` 获取数据，不能默认使用本地主仓库。
    pub fn get_snapshot_for_sync(&self, request: &SyncSnapshotRequest) -> Result<SyncResponse> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;
        let raw_ops = if request.peer_id == self.local_peer_id {
            local_snapshot_ops(self, request)?
        } else {
            shadow_snapshot_ops(self, request)?
        };

        let mut ops = Vec::with_capacity(raw_ops.len());
        let mut encrypted_bytes = 0_u64;
        for (_global_seq, entry) in raw_ops {
            let encrypted = repo_key.encrypt(&entry, entry.peer_seq)?;
            encrypted_bytes = encrypted_bytes
                .checked_add(encrypted.ciphertext.len() as u64)
                .and_then(|size| size.checked_add(encrypted.nonce.len() as u64))
                .and_then(|size| size.checked_add(64))
                .ok_or_else(|| anyhow::anyhow!("sync_resource_limit: snapshot size overflow"))?;
            if encrypted_bytes > MAX_SYNC_FACT_BYTES_PER_PAYLOAD {
                anyhow::bail!(
                    "sync_resource_limit: encrypted snapshot exceeds {} bytes",
                    MAX_SYNC_FACT_BYTES_PER_PAYLOAD
                );
            }
            ops.push(encrypted);
        }

        let waterline = raw_ops_waterline(&ops);

        Ok(SyncResponse {
            peer_id: request.peer_id.clone(),
            repo_id: request.repo_id,
            range: None,
            waterline,
            ops,
        })
    }
}

fn local_snapshot_ops(
    engine: &SyncEngine,
    request: &SyncSnapshotRequest,
) -> Result<Vec<(u64, crate::models::LedgerEntry)>> {
    engine
        .repo
        .find_local_repo_name_by_id(request.repo_id)?
        .ok_or_else(|| anyhow::anyhow!("Local repo not found for UUID {}", request.repo_id))?;
    let waterline = engine.repo.get_local_peer_waterline(&request.repo_id)?;
    if waterline == crate::models::PeerFactSeq::ZERO {
        return Ok(Vec::new());
    }
    engine.repo.get_local_ops_in_range(
        &request.repo_id,
        &request.peer_id,
        crate::models::PeerFactSeq::ONE,
        waterline,
    )
}

fn shadow_snapshot_ops(
    engine: &SyncEngine,
    request: &SyncSnapshotRequest,
) -> Result<Vec<(u64, crate::models::LedgerEntry)>> {
    let max_seq = engine
        .repo
        .get_shadow_max_seq(&request.peer_id, &request.repo_id)?;
    if max_seq == crate::models::PeerFactSeq::ZERO {
        return Ok(Vec::new());
    }
    engine.repo.get_shadow_ops_in_range(
        &request.peer_id,
        &request.repo_id,
        crate::models::PeerFactSeq::ONE,
        max_seq,
    )
}

fn raw_ops_waterline(ops: &[crate::security::EncryptedOp]) -> crate::models::PeerFactSeq {
    ops.last()
        .map(|op| op.peer_seq)
        .unwrap_or(crate::models::PeerFactSeq::ZERO)
}
