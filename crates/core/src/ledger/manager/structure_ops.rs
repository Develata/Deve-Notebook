//! plan_ref:
//!   - 06_repository#tree-projection-contract

use crate::ledger::{RepoManager, node_ops, ops as ledger_ops};
use crate::models::{DocId, NodeId, PeerId, StructureOp};
use anyhow::Result;

use super::commit_structure_plan;
use super::dir_structure_plan;
use super::structure_projection;

impl RepoManager {
    pub fn apply_file_structure_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
        doc_id_hint: Option<DocId>,
        peer_label: &str,
    ) -> Result<(DocId, Vec<StructureOp>)> {
        let plan = commit_structure_plan::plan_file_upsert(self, repo_name, path, doc_id_hint)?;
        self.append_structure_ops_in_local_repo(repo_name, peer_label, &plan.ops)?;
        Ok((plan.doc_id, plan.ops))
    }

    pub fn apply_file_delete_structure_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
        doc_id_hint: Option<DocId>,
        peer_label: &str,
    ) -> Result<Option<(DocId, Vec<StructureOp>)>> {
        let Some(plan) = commit_structure_plan::plan_delete(self, repo_name, path, doc_id_hint)?
        else {
            return Ok(None);
        };
        self.append_structure_ops_in_local_repo(repo_name, peer_label, &plan.ops)?;
        Ok(Some((plan.doc_id, plan.ops)))
    }

    pub fn apply_dir_create_structure_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
        peer_label: &str,
    ) -> Result<(NodeId, Vec<StructureOp>)> {
        let plan = dir_structure_plan::plan_create(self, repo_name, path)?;
        self.append_structure_ops_in_local_repo(repo_name, peer_label, &plan.ops)?;
        Ok((plan.node_id, plan.ops))
    }

    pub fn apply_dir_rename_structure_in_local_repo(
        &self,
        repo_name: &str,
        old_path: &str,
        new_path: &str,
        peer_label: &str,
    ) -> Result<Option<(NodeId, Vec<StructureOp>)>> {
        let Some(plan) = dir_structure_plan::plan_rename(self, repo_name, old_path, new_path)?
        else {
            return Ok(None);
        };
        self.append_structure_ops_in_local_repo(repo_name, peer_label, &plan.ops)?;
        Ok(Some((plan.node_id, plan.ops)))
    }

    pub fn apply_dir_delete_structure_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
        peer_label: &str,
    ) -> Result<Option<(NodeId, Vec<StructureOp>)>> {
        let Some(plan) = dir_structure_plan::plan_delete(self, repo_name, path)? else {
            return Ok(None);
        };
        self.append_structure_ops_in_local_repo(repo_name, peer_label, &plan.ops)?;
        Ok(Some((plan.node_id, plan.ops)))
    }

    /// Invariants:
    /// - 结构事实必须先进入 Ledger，再更新 projection。
    /// - 业务侧不得直接绕过本入口写 metadata/path/tree。
    pub(super) fn append_structure_ops_in_local_repo(
        &self,
        repo_name: &str,
        peer_label: &str,
        ops: &[StructureOp],
    ) -> Result<()> {
        let peer_id = PeerId::new(peer_label);
        let repo_scope = ledger_ops::local_repo_scope(repo_name);
        self.run_on_local_repo(repo_name, |db| {
            let write_txn = db.begin_write()?;
            for op in ops {
                let timestamp = chrono::Utc::now().timestamp_millis();
                node_ops::append_generated_structure_op_to_txn(
                    &write_txn,
                    peer_id.clone(),
                    op.clone(),
                    timestamp,
                    &repo_scope,
                )?;
                structure_projection::apply_in_txn(&write_txn, op)?;
            }
            write_txn.commit()?;
            Ok(())
        })
    }
}

#[cfg(test)]
#[path = "structure_ops_test.rs"]
mod tests;
