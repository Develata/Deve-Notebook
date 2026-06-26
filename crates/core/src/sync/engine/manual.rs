// crates\core\src\sync\engine
//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::config::SyncMode;
use crate::ledger::ShadowPayload;
use crate::models::{LedgerEntry, PeerId, RepoId};
use crate::sync::buffer::PendingSyncPayload;
use crate::sync::protocol::SyncResponse;
use anyhow::{Result, bail};

use super::transfer::{entries_with_seq, new_contiguous_remote_ops};

struct DecryptedPendingPayload {
    kind: DecryptedPendingKind,
    peer_id: PeerId,
    repo_id: RepoId,
    entries: Vec<LedgerEntry>,
    max_seq: u64,
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
    pub fn buffer_remote_ops(&mut self, response: SyncResponse) {
        self.pending_ops.push(response);
    }

    /// 暂存从远端接收的快照 (Manual 模式)
    pub fn buffer_remote_snapshot(&mut self, response: SyncResponse) {
        self.pending_ops.push_snapshot(response);
    }

    /// 根据当前同步模式处理增量 payload。
    pub fn receive_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_ops(response);
        }
        let count = response.ops.len() as u64;
        self.buffer_remote_ops(response);
        Ok(count)
    }

    /// 根据当前同步模式处理快照 payload。
    pub fn receive_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_snapshot(response);
        }
        let count = response.ops.len() as u64;
        self.buffer_remote_snapshot(response);
        Ok(count)
    }

    /// 合并所有待处理的操作 (Manual 模式显式触发)
    pub fn merge_pending(&mut self) -> Result<u64> {
        let pending = self.pending_ops.clone_all();
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

        // 单调化归约（批内单 peer/repo）：最新且非陈旧的 snapshot 是整库 base；
        // 只应用 base 之后严格连续的增量 ops。陈旧 snapshot 与已应用 ops 必须幂等跳过，
        // 不得 reset newer shadow，也不得二次 append 已接收 remote seq。
        let current = self.version_vector.get(&peer_id);
        let newest_snapshot = decrypted
            .iter()
            .filter(|payload| matches!(payload.kind, DecryptedPendingKind::Snapshot))
            .filter(|payload| payload.max_seq > current)
            .max_by_key(|payload| payload.max_seq);
        let base_seq = newest_snapshot.map_or(current, |payload| payload.max_seq);
        let ops_decrypted: Vec<_> = decrypted
            .iter()
            .filter(|payload| matches!(payload.kind, DecryptedPendingKind::Ops))
            .flat_map(|payload| entries_with_seq(&payload.entries))
            .collect();
        let new_ops = new_contiguous_remote_ops(ops_decrypted, base_seq)?;

        if let Some(snapshot) = newest_snapshot {
            let mut payloads = vec![snapshot.as_shadow_payload()];
            if !new_ops.entries.is_empty() {
                payloads.push(ShadowPayload::Ops(&new_ops.entries));
            }
            self.repo
                .apply_remote_payloads(&peer_id, &repo_id, &payloads)?;
            self.version_vector
                .update(peer_id, base_seq.max(new_ops.max_seq));
        } else {
            if !new_ops.entries.is_empty() {
                self.repo.apply_remote_payloads(
                    &peer_id,
                    &repo_id,
                    &[ShadowPayload::Ops(&new_ops.entries)],
                )?;
            }
            if new_ops.max_seq > 0 {
                self.version_vector.update(peer_id, new_ops.max_seq);
            }
        }

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
            let mut max_seq = 0;
            for enc_op in &response.ops {
                let entry = repo_key.decrypt(enc_op)?;
                if matches!(kind, DecryptedPendingKind::Ops) && entry.seq != enc_op.seq {
                    bail!(
                        "Encrypted op seq mismatch: envelope {}, payload {}",
                        enc_op.seq,
                        entry.seq
                    );
                }
                entries.push(entry);
                max_seq = max_seq.max(enc_op.seq);
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

#[cfg(test)]
mod tests;
