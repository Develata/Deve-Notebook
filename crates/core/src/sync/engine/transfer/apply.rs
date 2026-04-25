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
        decrypt_remote_ops(repo_key, &response.ops)?;
        Ok(())
    }

    /// 应用快照 (清空旧数据并覆盖)。
    pub fn apply_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;

        let mut max_seq = 0u64;
        let mut decrypted = Vec::with_capacity(response.ops.len());
        for enc_op in &response.ops {
            let entry = repo_key.decrypt(enc_op)?;
            decrypted.push((enc_op.seq, entry));
            max_seq = max_seq.max(enc_op.seq);
        }

        self.repo
            .reset_shadow_repo(&response.peer_id, &response.repo_id)?;

        for (_seq, entry) in decrypted {
            self.repo
                .append_remote_op(&response.peer_id, &response.repo_id, &entry)?;
        }

        self.version_vector.set_exact(response.peer_id, max_seq);

        Ok(max_seq)
    }

    pub fn validate_remote_ops(&self, response: &SyncResponse) -> Result<()> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;
        decrypt_remote_ops(repo_key, &response.ops)?;
        Ok(())
    }

    /// 应用从远端接收的操作（增量模式）。
    pub fn apply_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;

        let decrypted = decrypt_remote_ops(repo_key, &response.ops)?;
        let max_seq = decrypted
            .iter()
            .map(|(seq, _entry)| *seq)
            .max()
            .unwrap_or(0);
        for (_seq, entry) in decrypted {
            self.repo
                .append_remote_op(&response.peer_id, &response.repo_id, &entry)?;
        }

        if max_seq > 0 {
            self.version_vector.update(response.peer_id, max_seq);
        }

        Ok(max_seq)
    }
}

fn decrypt_remote_ops(
    repo_key: &crate::security::RepoKey,
    ops: &[crate::security::EncryptedOp],
) -> Result<Vec<(u64, LedgerEntry)>> {
    let mut decrypted = Vec::with_capacity(ops.len());
    for enc_op in ops {
        decrypted.push((enc_op.seq, repo_key.decrypt(enc_op)?));
    }
    Ok(decrypted)
}
