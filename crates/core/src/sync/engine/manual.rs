// crates\core\src\sync\engine
//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::config::SyncMode;
use crate::ledger::ShadowPayload;
use crate::models::{LedgerEntry, PeerFactSeq, PeerId, RepoId};
use crate::sync::buffer::{PendingOpsBuffer, PendingSyncPayload};
use crate::sync::protocol::SyncResponse;
use anyhow::{Result, bail};

use super::transfer::{decrypt_remote_ops, validate_full_fact_replay, validate_incremental_range};

struct DecryptedPendingPayload {
    kind: DecryptedPendingKind,
    peer_id: PeerId,
    repo_id: RepoId,
    entries: Vec<LedgerEntry>,
    max_seq: PeerFactSeq,
}

#[derive(Clone, Copy)]
enum DecryptedPendingKind {
    Ops,
    Snapshot,
}

impl SyncEngine {
    /// 检查是否有待合并的操作 (Manual 模式)
    pub fn has_pending_ops(&self) -> bool {
        !self.pending_ops.is_empty()
    }

    /// 获取待合并操作的数量
    pub fn pending_ops_count(&self) -> usize {
        self.pending_ops.count()
    }

    /// 暂存从远端接收的操作 (Manual 模式)
    #[cfg(test)]
    pub(crate) fn buffer_remote_ops(&mut self, response: SyncResponse) -> Result<()> {
        self.pending_ops.push(response)
    }

    /// 暂存从远端接收的快照 (Manual 模式)
    #[cfg(test)]
    pub(crate) fn buffer_remote_snapshot(&mut self, response: SyncResponse) -> Result<()> {
        self.pending_ops.push_snapshot(response)
    }

    /// 根据当前同步模式处理增量 payload。
    pub fn receive_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_ops(response);
        }
        let admission = self.pending_ops.preflight(&response)?;
        self.validate_remote_ops(&response)?;
        let count = response.ops.len() as u64;
        self.pending_ops
            .push_admitted(PendingSyncPayload::Ops(response), admission);
        Ok(count)
    }

    /// 根据当前同步模式处理快照 payload。
    pub fn receive_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_snapshot(response);
        }
        let admission = self.pending_ops.preflight(&response)?;
        self.validate_remote_snapshot(&response)?;
        let count = response.ops.len() as u64;
        self.pending_ops
            .push_admitted(PendingSyncPayload::Snapshot(response), admission);
        Ok(count)
    }

    /// 合并所有待处理的操作 (Manual 模式显式触发)
    pub fn merge_pending(&mut self) -> Result<u64> {
        let pending = self.pending_ops.take();
        match self.merge_taken_pending(&pending) {
            Ok(total) => Ok(total),
            Err(error) => {
                self.pending_ops = pending;
                Err(error)
            }
        }
    }

    fn merge_taken_pending(&mut self, pending: &PendingOpsBuffer) -> Result<u64> {
        let total = pending.count() as u64;
        let mut decrypted = self.decrypt_pending_payloads(pending.payloads())?;

        if decrypted.is_empty() {
            return Ok(0);
        }

        let peer_id = decrypted[0].peer_id.clone();
        let repo_id = decrypted[0].repo_id;
        if decrypted
            .iter()
            .any(|payload| payload.peer_id != peer_id || payload.repo_id != repo_id)
        {
            bail!("Manual merge requires one peer/repo target to preserve atomicity");
        }

        // 批内单 peer/repo：选择最高 waterline snapshot 作为 base，但先证明所有其他
        // snapshot 都是它的相同前缀；增量按 peer_seq 排序并只允许完全相同的重复。
        // 最终由一个 shadow write transaction 对持久化前缀再次做 equality/continuity gate。
        let newest_snapshot_index = decrypted
            .iter()
            .enumerate()
            .filter(|(_index, payload)| matches!(payload.kind, DecryptedPendingKind::Snapshot))
            .max_by_key(|(_index, payload)| payload.max_seq)
            .map(|(index, _payload)| index);
        if let Some(newest_index) = newest_snapshot_index {
            let newest = &decrypted[newest_index];
            for snapshot in decrypted
                .iter()
                .filter(|payload| matches!(payload.kind, DecryptedPendingKind::Snapshot))
            {
                ensure_snapshot_prefix_matches(snapshot, newest)?;
            }
        }
        let newest_snapshot =
            newest_snapshot_index.map(|index| std::mem::take(&mut decrypted[index].entries));
        let ops_capacity = decrypted
            .iter()
            .filter(|payload| matches!(payload.kind, DecryptedPendingKind::Ops))
            .map(|payload| payload.entries.len())
            .sum();
        let mut pending_ops = Vec::with_capacity(ops_capacity);
        for payload in &mut decrypted {
            if matches!(payload.kind, DecryptedPendingKind::Ops) {
                pending_ops.append(&mut payload.entries);
            }
        }
        drop(decrypted);
        let canonical_ops = canonicalize_pending_ops(pending_ops)?;
        let mut payloads = Vec::with_capacity(2);
        if let Some(snapshot) = &newest_snapshot {
            payloads.push(ShadowPayload::Snapshot(snapshot));
        }
        if !canonical_ops.is_empty() {
            payloads.push(ShadowPayload::Ops(&canonical_ops));
        }
        let persisted_waterline = self
            .repo
            .apply_remote_payloads(&peer_id, &repo_id, &payloads)?;
        self.version_vector.set_exact(peer_id, persisted_waterline);

        Ok(total)
    }

    /// 清空待处理的操作 (丢弃不合并)
    pub fn clear_pending(&mut self) {
        self.pending_ops.clear();
    }

    fn decrypt_pending_payloads(
        &self,
        pending: &[PendingSyncPayload],
    ) -> Result<Vec<DecryptedPendingPayload>> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;
        let mut decrypted = Vec::with_capacity(pending.len());
        for item in pending {
            let (kind, response) = match item {
                PendingSyncPayload::Ops(response) => (DecryptedPendingKind::Ops, response),
                PendingSyncPayload::Snapshot(response) => {
                    (DecryptedPendingKind::Snapshot, response)
                }
            };
            let decoded = decrypt_remote_ops(repo_key, &response.peer_id, &response.ops)?;
            match kind {
                DecryptedPendingKind::Ops => validate_incremental_range(response.range, &decoded)?,
                DecryptedPendingKind::Snapshot => {
                    validate_full_fact_replay(response.waterline, &decoded)?
                }
            }
            let max_seq = decoded
                .last()
                .map(|(seq, _entry)| *seq)
                .unwrap_or(PeerFactSeq::ZERO);
            let entries = decoded.into_iter().map(|(_seq, entry)| entry).collect();
            decrypted.push(DecryptedPendingPayload {
                kind,
                peer_id: response.peer_id.clone(),
                repo_id: response.repo_id,
                entries,
                max_seq,
            });
        }
        Ok(decrypted)
    }
}

fn ensure_snapshot_prefix_matches(
    candidate: &DecryptedPendingPayload,
    newest: &DecryptedPendingPayload,
) -> Result<()> {
    let overlap = candidate.entries.len().min(newest.entries.len());
    if candidate.entries[..overlap] != newest.entries[..overlap] {
        bail!(
            "sequence_conflict: snapshots disagree within confirmed prefix at waterlines {} and {}",
            candidate.max_seq,
            newest.max_seq
        );
    }
    Ok(())
}

fn canonicalize_pending_ops(mut entries: Vec<LedgerEntry>) -> Result<Vec<LedgerEntry>> {
    entries.sort_by_key(|entry| entry.peer_seq);
    for pair in entries.windows(2) {
        if pair[0].peer_seq == pair[1].peer_seq && pair[0] != pair[1] {
            bail!(
                "sequence_conflict: pending facts disagree at peer_seq {}",
                pair[1].peer_seq
            );
        }
    }
    entries.dedup_by(|current, previous| current.peer_seq == previous.peer_seq);
    Ok(entries)
}

#[cfg(test)]
mod tests;
