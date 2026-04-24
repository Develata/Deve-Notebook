//! plan_ref:
//!   - 06_repository#tree-projection-contract

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
        self.run_on_repo_db(repo_type, |db| {
            ops::get_structure_ops_for_node_from_db(db, node_id)
        })
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
