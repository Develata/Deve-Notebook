use crate::ledger::database_cache::evict_database_paths_under;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::schema::{
    CLIENT_OP_INDEX, DOC_OPS, DOCID_TO_PATH, INODE_TO_DOCID, INODE_TO_NODEID, LEDGER_OPS, NODE_OPS,
    NODE_PEER_SEQ, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID, PEER_DOC_SEQ, SNAPSHOT_DATA,
    SNAPSHOT_INDEX,
};
use crate::models::{DocId, NodeId, PeerId, RepoId};
use anyhow::{Context, Result};

impl RepoManager {
    pub fn repair_local_repo_catalog(&self) -> Result<()> {
        Self::repair_local_repo_metadata(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
            self.vault_root.as_deref(),
        )
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

    /// 整库重置指定 Shadow Repo 的 ledger/projection 内容，但保留 RepoMetadata。
    ///
    /// Invariants:
    /// - Snapshot 覆盖必须先物理清空旧 shadow 内容，再按原始 Ledger Facts 重放。
    /// - 只保留 `REPO_METADATA`，其余运行时表一律重建。
    pub fn reset_shadow_repo(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<()> {
        self.ensure_shadow_db(peer_id, repo_id)?;

        let guard = self.shadow_dbs.read().unwrap();
        let peer_map = guard
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer DBs not loaded"))?;
        let db = peer_map
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found"))?;

        let write_txn = db.begin_write()?;
        let _ = write_txn.delete_table(LEDGER_OPS)?;
        let _ = write_txn.delete_multimap_table(DOC_OPS)?;
        let _ = write_txn.delete_multimap_table(NODE_OPS)?;
        let _ = write_txn.delete_table(CLIENT_OP_INDEX)?;
        let _ = write_txn.delete_table(PEER_DOC_SEQ)?;
        let _ = write_txn.delete_table(NODE_PEER_SEQ)?;
        let _ = write_txn.delete_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.delete_table(SNAPSHOT_DATA)?;
        let _ = write_txn.delete_table(PATH_TO_DOCID)?;
        let _ = write_txn.delete_table(DOCID_TO_PATH)?;
        let _ = write_txn.delete_table(INODE_TO_DOCID)?;
        let _ = write_txn.delete_table(NODEID_TO_META)?;
        let _ = write_txn.delete_table(PATH_TO_NODEID)?;
        let _ = write_txn.delete_table(INODE_TO_NODEID)?;

        let _ = write_txn.open_table(LEDGER_OPS)?;
        let _ = write_txn.open_multimap_table(DOC_OPS)?;
        let _ = write_txn.open_multimap_table(NODE_OPS)?;
        let _ = write_txn.open_table(CLIENT_OP_INDEX)?;
        let _ = write_txn.open_table(PEER_DOC_SEQ)?;
        let _ = write_txn.open_table(NODE_PEER_SEQ)?;
        let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.open_table(SNAPSHOT_DATA)?;
        let _ = write_txn.open_table(PATH_TO_DOCID)?;
        let _ = write_txn.open_table(DOCID_TO_PATH)?;
        let _ = write_txn.open_table(INODE_TO_DOCID)?;
        let _ = write_txn.open_table(NODEID_TO_META)?;
        let _ = write_txn.open_table(PATH_TO_NODEID)?;
        let _ = write_txn.open_table(INODE_TO_NODEID)?;
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
        evict_database_paths_under(&peer_dir);

        // 3. Physical delete
        std::fs::remove_dir_all(&peer_dir)
            .with_context(|| format!("无法删除 Peer 目录: {:?}", peer_dir))?;

        tracing::info!("Deleted peer branch: {}", peer_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RepoManager;
    use crate::ledger::RepoInfo;
    use crate::models::PeerId;
    use tempfile::tempdir;

    #[test]
    fn delete_peer_branch_evicts_shadow_db_cache() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let repo = RepoManager::init(dir.path(), 8, Some("default"), Some("urn:default"))?;
        let peer_id = PeerId::new("peer-a");
        let first = RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:first".into()),
        };
        let second = RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:second".into()),
        };
        let path = repo
            .remotes_dir()
            .join(peer_id.to_filename())
            .join("wiki.redb");

        repo.ensure_shadow_repo_info(&peer_id, &first)?;
        assert!(path.exists());

        repo.delete_peer_branch(&peer_id)?;
        assert!(!path.exists());

        repo.ensure_shadow_repo_info(&peer_id, &second)?;
        assert!(path.exists());
        assert_eq!(
            repo.get_repo_info_for(Some(&peer_id), Some("wiki"))?
                .expect("recreated shadow repo")
                .uuid,
            second.uuid
        );
        Ok(())
    }
}
