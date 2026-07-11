// crates\core\src\sync\engine
//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::config::SyncMode;
use crate::ledger::ShadowPayload;
use crate::models::{LedgerEntry, PeerFactSeq, PeerId, RepoId};
use crate::sync::buffer::PendingSyncPayload;
use crate::sync::protocol::SyncResponse;
use anyhow::{Result, bail};

use super::transfer::entries_with_seq;

struct DecryptedPendingPayload {
    kind: DecryptedPendingKind,
    peer_id: PeerId,
    repo_id: RepoId,
    entries: Vec<LedgerEntry>,
    max_seq: PeerFactSeq,
    count: u64,
}

#[derive(Clone, Copy)]
enum DecryptedPendingKind {
    Ops,
    Snapshot,
}

impl DecryptedPendingPayload {
    fn as_shadow_payload(&self) -> ShadowPayload<'_> {
        match self.kind {
            DecryptedPendingKind::Ops => ShadowPayload::Ops(&self.entries),
            DecryptedPendingKind::Snapshot => ShadowPayload::Snapshot(&self.entries),
        }
    }
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
    pub(crate) fn buffer_remote_ops(&mut self, response: SyncResponse) {
        self.pending_ops.push(response);
    }

    /// 暂存从远端接收的快照 (Manual 模式)
    pub(crate) fn buffer_remote_snapshot(&mut self, response: SyncResponse) {
        self.pending_ops.push_snapshot(response);
    }

    /// 根据当前同步模式处理增量 payload。
    pub fn receive_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_ops(response);
        }
        self.validate_remote_ops(&response)?;
        let count = response.ops.len() as u64;
        self.buffer_remote_ops(response);
        Ok(count)
    }

    /// 根据当前同步模式处理快照 payload。
    pub fn receive_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_snapshot(response);
        }
        self.validate_remote_snapshot(&response)?;
        let count = response.ops.len() as u64;
        self.buffer_remote_snapshot(response);
        Ok(count)
    }

    /// 合并所有待处理的操作 (Manual 模式显式触发)
    pub fn merge_pending(&mut self) -> Result<u64> {
        let pending = self.pending_ops.clone_all();
        for item in &pending {
            match item {
                PendingSyncPayload::Ops(response) => self.validate_remote_ops(response)?,
                PendingSyncPayload::Snapshot(response) => {
                    self.validate_remote_snapshot(response)?
                }
            }
        }
        let decrypted = self.decrypt_pending_payloads(&pending)?;
        let total = decrypted.iter().map(|payload| payload.count).sum();

        if decrypted.is_empty() {
            self.pending_ops.clear();
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
        let newest_snapshot = decrypted
            .iter()
            .filter(|payload| matches!(payload.kind, DecryptedPendingKind::Snapshot))
            .max_by_key(|payload| payload.max_seq);
        if let Some(newest) = newest_snapshot {
            for snapshot in decrypted
                .iter()
                .filter(|payload| matches!(payload.kind, DecryptedPendingKind::Snapshot))
            {
                ensure_snapshot_prefix_matches(snapshot, newest)?;
            }
        }
        let ops_decrypted: Vec<_> = decrypted
            .iter()
            .filter(|payload| matches!(payload.kind, DecryptedPendingKind::Ops))
            .flat_map(|payload| entries_with_seq(&payload.entries))
            .collect();
        let canonical_ops = canonicalize_pending_ops(ops_decrypted)?;
        let mut payloads = Vec::with_capacity(2);
        if let Some(snapshot) = newest_snapshot {
            payloads.push(snapshot.as_shadow_payload());
        }
        if !canonical_ops.is_empty() {
            payloads.push(ShadowPayload::Ops(&canonical_ops));
        }
        let persisted_waterline = self
            .repo
            .apply_remote_payloads(&peer_id, &repo_id, &payloads)?;
        self.version_vector.set_exact(peer_id, persisted_waterline);

        self.pending_ops.clear();
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
            let mut entries = Vec::with_capacity(response.ops.len());
            let mut max_seq = PeerFactSeq::ZERO;
            for enc_op in &response.ops {
                let entry = repo_key.decrypt(enc_op)?;
                if matches!(kind, DecryptedPendingKind::Ops) && entry.peer_seq != enc_op.peer_seq {
                    bail!(
                        "Encrypted op seq mismatch: envelope {}, payload {}",
                        enc_op.peer_seq,
                        entry.peer_seq
                    );
                }
                entries.push(entry);
                max_seq = max_seq.max(enc_op.peer_seq);
            }
            decrypted.push(DecryptedPendingPayload {
                kind,
                peer_id: response.peer_id.clone(),
                repo_id: response.repo_id,
                count: response.ops.len() as u64,
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

fn canonicalize_pending_ops(
    mut entries: Vec<(PeerFactSeq, LedgerEntry)>,
) -> Result<Vec<LedgerEntry>> {
    entries.sort_by_key(|(seq, _entry)| *seq);
    let mut canonical: Vec<LedgerEntry> = Vec::with_capacity(entries.len());
    for (_seq, entry) in entries {
        if let Some(previous) = canonical.last()
            && previous.peer_seq == entry.peer_seq
        {
            if previous != &entry {
                bail!(
                    "sequence_conflict: pending facts disagree at peer_seq {}",
                    entry.peer_seq
                );
            }
            continue;
        }
        canonical.push(entry);
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests;
