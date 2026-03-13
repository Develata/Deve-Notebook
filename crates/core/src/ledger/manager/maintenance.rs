use crate::ledger::manager::types::RepoManager;
use crate::ledger::schema::{DOC_OPS, NODE_OPS};
use crate::models::{DocId, NodeId, PeerId, RepoId};
use anyhow::{Context, Result};

impl RepoManager {
    pub fn repair_local_repo_catalog(&self) -> Result<()> {
        Self::repair_local_repo_metadata(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
            self.vault_root.as_deref(),
        )?;
        self.repair_remote_repo_catalogs()
    }

    pub fn repair_remote_repo_catalogs(&self) -> Result<()> {
        let remotes_dir = self.remotes_dir();
        if !remotes_dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(remotes_dir)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if name.starts_with('.') || name.is_empty() {
                continue;
            }
            self.scan_remote_repo_entries(&PeerId::new(name))?;
        }
        Ok(())
    }

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

    /// 重置指定 Shadow 节点的结构事实索引 (物理清空)
    pub fn reset_shadow_node(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        node_id: &NodeId,
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
        write_txn
            .open_multimap_table(NODE_OPS)?
            .remove_all(&node_id.as_u128())?;
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
