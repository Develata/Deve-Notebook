use crate::ledger::manager::types::RepoManager;
use crate::ledger::schema::DOC_OPS;
use crate::models::{DocId, PeerId, RepoId};
use anyhow::{Context, Result};

impl RepoManager {
    /// 重置指定 Shadow 文档的所有历史操作 (物理清空)
    ///
    /// **用途**: 当接收到 P2P Snapshot 时，旧的增量日志失效，需清空并重写。
    pub fn reset_shadow_doc(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: &DocId,
    ) -> Result<()> {
        self.ensure_shadow_db(peer_id, repo_id)?;

        let guard = self.shadow_dbs.read().unwrap();
        let peer_map = guard
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer DBs not loaded"))?;
        let db = peer_map
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found"))?;

        let write_txn = db.begin_write()?;

        {
            let mut table = write_txn.open_multimap_table(DOC_OPS)?;
            // Redb multimap remove deletes a specific key-value pair.
            // remove_all is what we want (delete all values for a key).
            table.remove_all(&doc_id.as_u128())?;
        }

        write_txn.commit()?;
        Ok(())
    }

    /// 删除指定 Peer 的影子库目录
    pub fn delete_peer_branch(&self, peer_id: &PeerId) -> Result<()> {
        let peer_dir = self.remotes_dir().join(peer_id.to_filename());

        // 1. Check if exists
        if !peer_dir.exists() {
            return Ok(()); // Idempotent success
        }

        // 2. Remove from cache (shadow_dbs)
        {
            let mut guard = self.shadow_dbs.write().unwrap();
            guard.remove(peer_id);
        }

        // 3. Physical delete
        std::fs::remove_dir_all(&peer_dir)
            .with_context(|| format!("无法删除 Peer 目录: {:?}", peer_dir))?;

        tracing::info!("Deleted peer branch: {}", peer_id);
        Ok(())
    }
}
