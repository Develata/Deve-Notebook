//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::models::{LedgerEntry, PeerFactSeq, PeerId};
use crate::sync::protocol::SyncResponse;
use anyhow::Result;

impl SyncEngine {
    pub fn validate_remote_snapshot(&self, response: &SyncResponse) -> Result<()> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;
        let decrypted = decrypt_remote_ops(repo_key, &response.peer_id, &response.ops)?;
        validate_full_fact_replay(response.waterline, &decrypted)?;
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

        let decrypted = decrypt_remote_ops(repo_key, &response.peer_id, &response.ops)?;
        validate_full_fact_replay(response.waterline, &decrypted)?;

        let entries = decrypted_entries(decrypted);
        let persisted_waterline =
            self.repo
                .replace_shadow_repo_ops(&response.peer_id, &response.repo_id, &entries)?;
        self.version_vector
            .set_exact(response.peer_id, persisted_waterline);

        Ok(persisted_waterline)
    }

    pub fn validate_remote_ops(&self, response: &SyncResponse) -> Result<()> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;
        let decrypted = decrypt_remote_ops(repo_key, &response.peer_id, &response.ops)?;
        validate_incremental_range(response.range, &decrypted)?;
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

        let decrypted = decrypt_remote_ops(repo_key, &response.peer_id, &response.ops)?;
        validate_incremental_range(response.range, &decrypted)?;
        let entries = decrypted_entries(decrypted);
        let persisted_waterline =
            self.repo
                .append_remote_ops(&response.peer_id, &response.repo_id, &entries)?;
        self.version_vector
            .set_exact(response.peer_id, persisted_waterline);
        Ok(persisted_waterline)
    }
}

fn decrypt_remote_ops(
    repo_key: &crate::security::RepoKey,
    source_peer_id: &PeerId,
    ops: &[crate::security::EncryptedOp],
) -> Result<Vec<(PeerFactSeq, LedgerEntry)>> {
    let mut decrypted = Vec::with_capacity(ops.len());
    for enc_op in ops {
        let entry = repo_key.decrypt(enc_op)?;
        if entry.peer_seq != enc_op.peer_seq {
            anyhow::bail!(
                "Encrypted op seq mismatch: envelope {}, payload {}",
                enc_op.peer_seq,
                entry.peer_seq
            );
        }
        if entry.origin_peer_id != *source_peer_id {
            anyhow::bail!(
                "Remote op origin mismatch: source {}, payload {} at peer_seq {}",
                source_peer_id,
                entry.origin_peer_id,
                entry.peer_seq
            );
        }
        decrypted.push((enc_op.peer_seq, entry));
    }
    Ok(decrypted)
}

pub(crate) fn entries_with_seq(entries: &[LedgerEntry]) -> Vec<(PeerFactSeq, LedgerEntry)> {
    entries
        .iter()
        .cloned()
        .map(|entry| (entry.peer_seq, entry))
        .collect()
}

fn decrypted_entries(decrypted: Vec<(PeerFactSeq, LedgerEntry)>) -> Vec<LedgerEntry> {
    decrypted.into_iter().map(|(_seq, entry)| entry).collect()
}

fn validate_incremental_range(
    range: Option<(PeerFactSeq, PeerFactSeq)>,
    decrypted: &[(PeerFactSeq, LedgerEntry)],
) -> Result<()> {
    let (start, end) = range.ok_or_else(|| anyhow::anyhow!("incremental sync range missing"))?;
    if start == PeerFactSeq::ZERO || end < start {
        anyhow::bail!("invalid incremental closed range {}..={}", start, end);
    }
    if !decrypted.is_empty() {
        validate_exact_sequence(start, end, decrypted)?;
    }
    let expected_len = end.get() - start.get() + 1;
    if decrypted.len() as u64 != expected_len {
        anyhow::bail!(
            "sequence_gap: range {}..={} expects {} facts, received {}",
            start,
            end,
            expected_len,
            decrypted.len()
        );
    }
    Ok(())
}

fn validate_full_fact_replay(
    waterline: PeerFactSeq,
    decrypted: &[(PeerFactSeq, LedgerEntry)],
) -> Result<()> {
    if waterline == PeerFactSeq::ZERO {
        if decrypted.is_empty() {
            return Ok(());
        }
        anyhow::bail!("snapshot waterline is zero but payload is not empty");
    }
    if decrypted.len() as u64 != waterline.get() {
        anyhow::bail!(
            "sequence_gap: full-fact replay waterline {} expects {} facts, received {}",
            waterline,
            waterline,
            decrypted.len()
        );
    }
    validate_exact_sequence(PeerFactSeq::ONE, waterline, decrypted)
}

fn validate_exact_sequence(
    start: PeerFactSeq,
    end: PeerFactSeq,
    decrypted: &[(PeerFactSeq, LedgerEntry)],
) -> Result<()> {
    let mut expected = start;
    for (seq, _entry) in decrypted {
        if *seq != expected {
            anyhow::bail!(
                "non-contiguous remote ops: expected seq {}, received {}",
                expected,
                seq
            );
        }
        if *seq != end {
            expected = seq
                .next()
                .ok_or_else(|| anyhow::anyhow!("PeerFactSeq overflow after {}", seq))?;
        }
    }
    Ok(())
}
