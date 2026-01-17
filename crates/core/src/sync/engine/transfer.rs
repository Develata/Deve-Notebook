// crates\core\src\sync\engine
use crate::sync::protocol::{SyncRequest, SyncResponse};
use super::SyncEngine;
use crate::security::{RepoKey, EncryptedOp};
use anyhow::Result;

impl SyncEngine {
    /// 从本地仓库获取指定范围的操作 (用于发送给远端)
    /// **安全**: 使用 `RepoKey` 对 LedgerEntry 进行加密 (Envelope Pattern)。
    pub fn get_ops_for_sync(&self, request: &SyncRequest) -> Result<SyncResponse> {
        let raw_ops = if request.peer_id == self.local_peer_id {
            // 请求的是本地数据 - 从 Local Repo 获取
            self.repo.get_local_ops_in_range(&request.repo_id, request.range.0, request.range.1)?
        } else {
            // 请求的是远端数据 - 从 Shadow Repo 获取
            self.repo.get_shadow_ops_in_range(&request.peer_id, &request.repo_id, request.range.0, request.range.1)?
        };

        let repo_key = self.repo_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot encrypt ops"))?;

        let mut encrypted_ops = Vec::with_capacity(raw_ops.len());
        for (seq, entry) in raw_ops {
            let mut enc_op = repo_key.encrypt(&entry)?;
            enc_op.seq = seq; // Fill sequence number which is plain for ordering
            encrypted_ops.push(enc_op);
        }

        Ok(SyncResponse {
            peer_id: request.peer_id.clone(),
            repo_id: request.repo_id,
            ops: encrypted_ops,
        })
    }

    /// 应用从远端接收的操作
    /// **安全**: 先解密，再写入 Shadow DB。
    pub fn apply_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        let mut max_seq = 0u64;
        
        let repo_key = self.repo_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;

        for enc_op in response.ops {
            let seq = enc_op.seq;
            let entry = repo_key.decrypt(&enc_op)?;
            
            // Write Decrypted (Plaintext) Entry to Shadow DB
            self.repo.append_remote_op(&response.peer_id, &response.repo_id, &entry)?;
            max_seq = max_seq.max(seq);
        }

        // 更新 Version Vector
        if max_seq > 0 {
            self.version_vector.update(response.peer_id, max_seq);
        }

        Ok(max_seq)
    }
}
