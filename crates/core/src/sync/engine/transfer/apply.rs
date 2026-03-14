use super::SyncEngine;
use crate::sync::protocol::SyncResponse;
use anyhow::Result;

impl SyncEngine {
    /// 应用快照 (清空旧数据并覆盖)。
    pub fn apply_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;

        let mut max_seq = 0u64;
        self.repo
            .reset_shadow_repo(&response.peer_id, &response.repo_id)?;

        for enc_op in response.ops {
            let seq = enc_op.seq;
            let entry = repo_key.decrypt(&enc_op)?;

            self.repo
                .append_remote_op(&response.peer_id, &response.repo_id, &entry)?;
            max_seq = max_seq.max(seq);
        }

        self.version_vector.set_exact(response.peer_id, max_seq);

        Ok(max_seq)
    }

    /// 应用从远端接收的操作（增量模式）。
    pub fn apply_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;

        let mut max_seq = 0u64;
        for enc_op in response.ops {
            let seq = enc_op.seq;
            let entry = repo_key.decrypt(&enc_op)?;
            self.repo
                .append_remote_op(&response.peer_id, &response.repo_id, &entry)?;
            max_seq = max_seq.max(seq);
        }

        if max_seq > 0 {
            self.version_vector.update(response.peer_id, max_seq);
        }

        Ok(max_seq)
    }
}
