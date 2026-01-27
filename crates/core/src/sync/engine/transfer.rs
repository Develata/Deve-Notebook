// crates\core\src\sync\engine
use super::SyncEngine;
use crate::sync::protocol::{SyncRequest, SyncResponse};

use anyhow::Result;

impl SyncEngine {
    /// 从本地仓库获取指定范围的操作 (用于发送给远端)
    /// **安全**: 使用 `RepoKey` 对 LedgerEntry 进行加密 (Envelope Pattern)。
    pub fn get_ops_for_sync(&self, request: &SyncRequest) -> Result<SyncResponse> {
        let raw_ops = if request.peer_id == self.local_peer_id {
            // 请求的是本地数据 - 从 Local Repo 获取
            self.repo
                .get_local_ops_in_range(&request.repo_id, request.range.0, request.range.1)?
        } else {
            // 请求的是远端数据 - 从 Shadow Repo 获取
            self.repo.get_shadow_ops_in_range(
                &request.peer_id,
                &request.repo_id,
                request.range.0,
                request.range.1,
            )?
        };

        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot encrypt ops"))?;

        let mut encrypted_ops = Vec::with_capacity(raw_ops.len());
        for (seq, entry) in raw_ops {
            let enc_op = repo_key.encrypt(&entry, seq)?;
            // enc_op.seq is now set by encrypt
            encrypted_ops.push(enc_op);
        }

        Ok(SyncResponse {
            peer_id: request.peer_id.clone(),
            repo_id: request.repo_id,
            ops: encrypted_ops,
        })
    }

    /// 获取快照数据 (用于全量同步)
    ///
    /// **逻辑**:
    /// 1. 遍历所有文档。
    /// 2. 重建最新文本。
    /// 3. 生成 Op::Insert (pos:0)。
    /// 4. 加密并打包。
    pub fn get_snapshot_for_sync(
        &self,
        request: &crate::sync::protocol::SyncSnapshotRequest,
    ) -> Result<crate::sync::protocol::SyncResponse> {
        // TODO: Handle request.repo_id correctly to select DB
        let repo_name = self.repo.local_repo_name(); // Currently only supports main repo
        let mut ops = Vec::new();

        // 1. List all docs
        let docs = self.repo.list_local_docs(Some(repo_name))?;

        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;

        for (doc_id, _) in docs {
            // 2. Reconstruct content
            // We use get_local_ops to get history and reconstruct.
            // Or get_committed_content if we have snapshots.
            // To ensure "latest state", we reconstruct.
            let entries = self.repo.get_local_ops(doc_id)?;
            let ops_vec: Vec<_> = entries.iter().map(|(_, e)| e.clone()).collect();
            let content = crate::state::reconstruct_content(&ops_vec);

            if content.is_empty() {
                continue;
            }

            // 3. Create Snapshot Op
            let op = crate::models::Op::Insert { pos: 0, content };

            // 4. Create LedgerEntry (Seq is irrelevant for snapshot, but we use max seq?)
            // Actually seq matters for VV update. We should use the latest seq we have for this doc?
            // Or just use 0? If we use 0, remote VV won't update correctly.
            // We should use the current Max Seq of the local repo to ensure remote VV catches up!
            // But wait, VV is per-peer.
            // If we send a snapshot, we are essentially saying "Here is the state at MySeq=X".
            // So we should attach the *latest local sequence number* to these ops.

            let latest_seq = entries.last().map(|(s, _)| *s).unwrap_or(0);

            let entry = crate::models::LedgerEntry {
                doc_id,
                op,
                timestamp: chrono::Utc::now().timestamp_millis(),
                peer_id: self.local_peer_id.clone(),
                seq: latest_seq,
            };

            // 5. Encrypt
            let enc_op = repo_key.encrypt(&entry, latest_seq)?;
            ops.push(enc_op);
        }

        Ok(SyncResponse {
            peer_id: self.local_peer_id.clone(),
            repo_id: request.repo_id,
            ops,
        })
    }

    /// 应用快照 (清空旧数据并覆盖)
    pub fn apply_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        let mut max_seq = 0u64;

        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured"))?;

        // Group ops by doc_id to minimize resets
        // But since we just get a list, we can just iterate.
        // Wait, we need to reset the doc BEFORE appending.
        // We can maintain a set of "reset doc ids" in this transaction.
        let mut reset_docs = std::collections::HashSet::new();

        for enc_op in response.ops {
            let seq = enc_op.seq;
            let entry = repo_key.decrypt(&enc_op)?;

            // 1. Reset if first time seeing this doc in this snapshot batch
            if !reset_docs.contains(&entry.doc_id) {
                self.repo
                    .reset_shadow_doc(&response.peer_id, &response.repo_id, &entry.doc_id)?;
                reset_docs.insert(entry.doc_id);
            }

            // 2. Write Snapshot Op
            self.repo
                .append_remote_op(&response.peer_id, &response.repo_id, &entry)?;

            max_seq = max_seq.max(seq);
        }

        // 更新 Version Vector
        if max_seq > 0 {
            self.version_vector.update(response.peer_id, max_seq);
        }

        Ok(max_seq)
    }

    /// 应用从远端接收的操作

    /// **安全**: 先解密，再写入 Shadow DB。
    pub fn apply_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        let mut max_seq = 0u64;

        let repo_key = self
            .repo_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("RepoKey not configured, cannot decrypt ops"))?;

        for enc_op in response.ops {
            let seq = enc_op.seq;
            let entry = repo_key.decrypt(&enc_op)?;

            // Write Decrypted (Plaintext) Entry to Shadow DB
            self.repo
                .append_remote_op(&response.peer_id, &response.repo_id, &entry)?;
            max_seq = max_seq.max(seq);
        }

        // 更新 Version Vector
        if max_seq > 0 {
            self.version_vector.update(response.peer_id, max_seq);
        }

        Ok(max_seq)
    }
}
