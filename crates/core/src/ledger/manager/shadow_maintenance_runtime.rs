//! plan_ref:
//!   - 05_network#remote-shadow-apply-atomicity
//!   - 04_storage#repo-runtime-layout

use crate::ledger::database_cache::evict_database_paths_under;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::schema::{
    CLIENT_OP_INDEX, DOC_OPS, DOCID_TO_PATH, INODE_TO_DOCID, INODE_TO_NODEID, LEDGER_OPS, NODE_OPS,
    NODE_PEER_SEQ, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID, PEER_DOC_SEQ, SNAPSHOT_DATA,
    SNAPSHOT_INDEX,
};
use crate::models::{DocId, NodeId, PeerId, RepoId};
use anyhow::{Context, Result};

pub(crate) struct ShadowMaintenanceRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> ShadowMaintenanceRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    /// 重置指定 Shadow 文档的所有历史操作。
    pub(crate) fn reset_shadow_doc(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: &DocId,
    ) -> Result<()> {
        self.manager.ensure_shadow_db(peer_id, repo_id)?;

        let guard = self.manager.read_shadow_dbs()?;
        let peer_map = guard
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer DBs not loaded"))?;
        let db = peer_map
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found"))?;

        let write_txn = db.begin_write()?;
        write_txn
            .open_multimap_table(DOC_OPS)?
            .remove_all(&doc_id.as_u128())?;
        write_txn.commit()?;
        Ok(())
    }

    /// 重置指定 Shadow 节点的结构事实索引。
    pub(crate) fn reset_shadow_node(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        node_id: &NodeId,
    ) -> Result<()> {
        self.manager.ensure_shadow_db(peer_id, repo_id)?;

        let guard = self.manager.read_shadow_dbs()?;
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
    pub(crate) fn reset_shadow_repo(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<()> {
        self.manager.ensure_shadow_db(peer_id, repo_id)?;

        let guard = self.manager.read_shadow_dbs()?;
        let peer_map = guard
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer DBs not loaded"))?;
        let db = peer_map
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found"))?;

        let write_txn = db.begin_write()?;
        Self::reset_shadow_repo_txn(&write_txn)?;
        write_txn.commit()?;
        Ok(())
    }

    pub(crate) fn reset_shadow_repo_txn(write_txn: &redb::WriteTransaction) -> Result<()> {
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
        Ok(())
    }

    /// 删除指定 Peer 的影子库目录。
    pub(crate) fn delete_peer_branch(&self, peer_id: &PeerId) -> Result<()> {
        let peer_dir = self.manager.remotes_dir().join(peer_id.to_filename());

        match peer_dir.try_exists() {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("无法检测 Peer 目录是否存在: {:?}", peer_dir));
            }
        }

        {
            let mut guard = self.manager.write_shadow_dbs()?;
            guard.remove(peer_id);
        }
        evict_database_paths_under(&peer_dir)?;

        std::fs::remove_dir_all(&peer_dir)
            .with_context(|| format!("无法删除 Peer 目录: {:?}", peer_dir))?;

        tracing::info!("Deleted peer branch: {}", peer_id);
        Ok(())
    }
}

impl RepoManager {
    pub fn reset_shadow_doc(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: &DocId,
    ) -> Result<()> {
        self.shadow_maintenance_runtime()
            .reset_shadow_doc(peer_id, repo_id, doc_id)
    }

    pub fn reset_shadow_node(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        node_id: &NodeId,
    ) -> Result<()> {
        self.shadow_maintenance_runtime()
            .reset_shadow_node(peer_id, repo_id, node_id)
    }

    pub fn reset_shadow_repo(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<()> {
        self.shadow_maintenance_runtime()
            .reset_shadow_repo(peer_id, repo_id)
    }

    pub(crate) fn reset_shadow_repo_txn(write_txn: &redb::WriteTransaction) -> Result<()> {
        ShadowMaintenanceRuntime::reset_shadow_repo_txn(write_txn)
    }

    pub fn delete_peer_branch(&self, peer_id: &PeerId) -> Result<()> {
        self.shadow_maintenance_runtime()
            .delete_peer_branch(peer_id)
    }

    pub(crate) fn shadow_maintenance_runtime(&self) -> ShadowMaintenanceRuntime<'_> {
        ShadowMaintenanceRuntime::new(self)
    }
}

#[cfg(test)]
mod tests;
