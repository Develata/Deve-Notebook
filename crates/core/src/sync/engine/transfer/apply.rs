//! plan_ref:
//!   - 05_network#server-ws-runtime

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
    pub fn apply_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;

        let decrypted = decrypt_remote_ops(repo_key, &response.ops, false)?;
        let max_seq = max_decrypted_seq(&decrypted);
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
    pub fn apply_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;

        let decrypted = decrypt_remote_ops(repo_key, &response.ops, true)?;
        let max_seq = decrypted
            .iter()
            .map(|(seq, _entry)| *seq)
            .max()
            .unwrap_or(0);
        let entries = decrypted_entries(decrypted);
        self.repo
            .append_remote_ops(&response.peer_id, &response.repo_id, &entries)?;

        if max_seq > 0 {
            self.version_vector.update(response.peer_id, max_seq);
        }

        Ok(max_seq)
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
