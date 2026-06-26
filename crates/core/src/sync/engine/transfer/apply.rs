//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::models::LedgerEntry;
use crate::sync::protocol::SyncResponse;
use anyhow::Result;

impl SyncEngine {
    pub fn validate_remote_snapshot(&self, response: &SyncResponse) -> Result<()> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;
        decrypt_remote_ops(repo_key, &response.ops, false)?;
        Ok(())
    }

    /// 应用快照 (清空旧数据并覆盖)。
    ///
    /// **单调性守卫**: 快照语义是「整库 reset 到该状态」。若快照的 max seq 不超过
    /// 我们已持有的该 peer vector，应用它只会清掉更 newer 的 ops 并回退 vector，
    /// 违反 plan `07_network#server-ws-runtime`「不得破坏 vector monotonicity」。
    /// 这类陈旧/乱序到达的快照直接跳过（既不 reset 影子库也不动 vector）。
    pub fn apply_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;

        let decrypted = decrypt_remote_ops(repo_key, &response.ops, false)?;
        let max_seq = max_decrypted_seq(&decrypted);

        let current = self.version_vector.get(&response.peer_id);
        if max_seq <= current {
            return Ok(current);
        }

        let entries = decrypted_entries(decrypted);

        self.repo
            .replace_shadow_repo_ops(&response.peer_id, &response.repo_id, &entries)?;

        self.version_vector.set_exact(response.peer_id, max_seq);

        Ok(max_seq)
    }

    pub fn validate_remote_ops(&self, response: &SyncResponse) -> Result<()> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;
        decrypt_remote_ops(repo_key, &response.ops, true)?;
        Ok(())
    }

    /// 应用从远端接收的操作（增量模式）。
    ///
    /// **连续性守卫**: 增量批次必须从 `current+1` 起无空洞地推进该 peer 的 vector。
    /// 若批次跳过了未接收的 seq，`update(peer, max_seq)` 会让 vector 越过空洞、令后续
    /// diff 不再请求缺失 op，造成静默丢失。检测到空洞时 fail-closed，不做任何写入，
    /// 留待重连/重新请求（与本文件 envelope/payload seq 失配的 fail-closed 一致）。
    pub fn apply_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;

        let current = self.version_vector.get(&response.peer_id);
        let new_ops =
            new_contiguous_remote_ops(decrypt_remote_ops(repo_key, &response.ops, true)?, current)?;
        if !new_ops.entries.is_empty() {
            self.repo
                .append_remote_ops(&response.peer_id, &response.repo_id, &new_ops.entries)?;
        }

        if new_ops.max_seq > 0 {
            self.version_vector
                .update(response.peer_id, new_ops.max_seq);
        }

        Ok(new_ops.max_seq)
    }
}

fn decrypt_remote_ops(
    repo_key: &crate::security::RepoKey,
    ops: &[crate::security::EncryptedOp],
    validate_entry_seq: bool,
) -> Result<Vec<(u64, LedgerEntry)>> {
    let mut decrypted = Vec::with_capacity(ops.len());
    for enc_op in ops {
        let entry = repo_key.decrypt(enc_op)?;
        if validate_entry_seq && entry.seq != enc_op.seq {
            anyhow::bail!(
                "Encrypted op seq mismatch: envelope {}, payload {}",
                enc_op.seq,
                entry.seq
            );
        }
        decrypted.push((enc_op.seq, entry));
    }
    Ok(decrypted)
}

pub(crate) struct NewRemoteOps {
    pub(crate) entries: Vec<LedgerEntry>,
    pub(crate) max_seq: u64,
}

/// 过滤并校验解密后的增量 op 批次相对当前 vector 无空洞。
///
/// 与已应用区间重叠的 `seq <= current` 必须幂等跳过，不能再次 append 到 shadow。
/// `current` 之上的 seq 必须从 `current+1` 起严格连续；出现空洞或重复即返回结构化错误，
/// 调用方不写入任何状态。
pub(crate) fn new_contiguous_remote_ops(
    decrypted: Vec<(u64, LedgerEntry)>,
    current: u64,
) -> Result<NewRemoteOps> {
    let Some(mut expected) = current.checked_add(1) else {
        return Ok(NewRemoteOps {
            entries: Vec::new(),
            max_seq: 0,
        });
    };
    let mut pending: Vec<_> = decrypted
        .into_iter()
        .filter(|(seq, _entry)| *seq > current)
        .collect();
    pending.sort_unstable_by_key(|(seq, _entry)| *seq);

    let mut entries = Vec::with_capacity(pending.len());
    let mut last_seq = None;
    let mut max_seq = 0;
    for (seq, entry) in pending {
        if last_seq == Some(seq) {
            anyhow::bail!("duplicate remote op seq {seq}");
        }
        if seq != expected {
            anyhow::bail!("non-contiguous remote ops: expected seq {expected}, received {seq}");
        }
        max_seq = seq;
        entries.push(entry);
        last_seq = Some(seq);
        expected = seq.saturating_add(1);
    }
    Ok(NewRemoteOps { entries, max_seq })
}

pub(crate) fn entries_with_seq(entries: &[LedgerEntry]) -> Vec<(u64, LedgerEntry)> {
    entries
        .iter()
        .cloned()
        .map(|entry| (entry.seq, entry))
        .collect()
}

pub(crate) fn max_decrypted_seq(decrypted: &[(u64, LedgerEntry)]) -> u64 {
    decrypted
        .iter()
        .map(|(seq, _entry)| *seq)
        .max()
        .unwrap_or(0)
}

fn decrypted_entries(decrypted: Vec<(u64, LedgerEntry)>) -> Vec<LedgerEntry> {
    decrypted.into_iter().map(|(_seq, entry)| entry).collect()
}
