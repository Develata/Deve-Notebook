use crate::ledger::RepoManager;
use crate::ledger::ops;
use crate::models::{LedgerEntry, NodeId, RepoType};
use anyhow::Result;

impl RepoManager {
    pub fn get_structure_ops(
        &self,
        repo_type: &RepoType,
        node_id: NodeId,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        match repo_type {
            RepoType::Local(_) => ops::get_structure_ops_for_node_from_db(&self.local_db, node_id),
            RepoType::Remote(peer_id, repo_id) => {
                self.ensure_shadow_db(peer_id, repo_id)?;
                let dbs = self.shadow_dbs.read().unwrap();
                let peer_repos = dbs
                    .get(peer_id)
                    .ok_or_else(|| anyhow::anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
                let db = peer_repos.get(repo_id).ok_or_else(|| {
                    anyhow::anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id)
                })?;
                ops::get_structure_ops_for_node_from_db(db, node_id)
            }
        }
    }

    pub fn get_local_structure_ops(&self, node_id: NodeId) -> Result<Vec<(u64, LedgerEntry)>> {
        ops::get_structure_ops_for_node_from_db(&self.local_db, node_id)
    }

    pub fn get_local_structure_ops_in_local_repo(
        &self,
        repo_name: &str,
        node_id: NodeId,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_local_repo(repo_name, |db| {
            ops::get_structure_ops_for_node_from_db(db, node_id)
        })
    }
}
