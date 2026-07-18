//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/projection#projection-contract
//!
//! Atomic application of preflighted External Changes into ledger authority.

use crate::ledger::manager::commit_plan::CommitTarget;
use crate::ledger::manager::prepared_change_batch::{
    PreparedLedgerChangeBatch, PreparedLedgerTarget, PreparedLedgerUpsert,
};
use crate::ledger::{RepoManager, reconcile};
use crate::models::FileNodeId;
use crate::source_control::staging::StagedEntry;
use crate::source_control::{ExternalApplyOutcome, PreparedExternalApply};
use crate::utils::fs::checked_exists;
use anyhow::{Result, anyhow};

use super::{commit_structure_plan, prepared_change_batch};

impl RepoManager {
    pub(super) fn prepare_external_apply_in_local_repo(
        &self,
        repo_name: &str,
        targets: Vec<CommitTarget>,
        staged_snapshot: Vec<(String, StagedEntry)>,
        expected_ledger_head: u64,
        repo_id: crate::models::RepoId,
    ) -> Result<PreparedExternalApply> {
        let prepared = targets
            .iter()
            .map(|target| self.prepare_external_target(repo_name, target))
            .collect::<Result<Vec<_>>>()?;
        let mut changed_paths: Vec<_> = staged_snapshot
            .iter()
            .map(|(path, _)| path.clone())
            .collect();
        changed_paths.sort();
        changed_paths.dedup();
        Ok(PreparedExternalApply::from_batch(
            PreparedLedgerChangeBatch::external(
                repo_id,
                expected_ledger_head,
                staged_snapshot,
                prepared,
                changed_paths,
            ),
        ))
    }

    pub(super) fn commit_prepared_external_apply_in_local_repo(
        &self,
        repo_name: &str,
        prepared: PreparedExternalApply,
    ) -> Result<ExternalApplyOutcome> {
        prepared_change_batch::commit_external(self, repo_name, prepared.into_batch())
    }

    fn prepare_external_target(
        &self,
        repo_name: &str,
        target: &CommitTarget,
    ) -> Result<PreparedLedgerTarget> {
        if target.delete_only {
            return Ok(PreparedLedgerTarget::Delete {
                path: target.path.clone(),
                doc_id: target.doc_id,
            });
        }

        let content = target.validated_content.as_deref().ok_or_else(|| {
            anyhow!(
                "external apply target was not preflighted with content: {}",
                target.path
            )
        })?;
        let doc_id =
            commit_structure_plan::resolve_doc_id(self, repo_name, &target.path, target.doc_id)?;
        let existing_ops = self.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let entries: Vec<_> = existing_ops.into_iter().map(|(_, entry)| entry).collect();
        let content_ops = reconcile::compute_reconcile_patch(&entries, content)?;
        let inode = self.external_target_inode(repo_name, &target.path)?;
        Ok(PreparedLedgerTarget::Upsert(PreparedLedgerUpsert {
            path: target.path.clone(),
            doc_id,
            content_ops,
            inode,
        }))
    }

    fn external_target_inode(&self, repo_name: &str, path: &str) -> Result<Option<FileNodeId>> {
        let file_path = self.local_repo_workspace_path(repo_name, path)?;
        if !checked_exists(&file_path, "staged workspace path while binding inode")? {
            return Ok(None);
        }
        let file_id = file_id::get_file_id(&file_path)?;
        use crate::utils::hash::StableHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = StableHasher::new();
        file_id.hash(&mut hasher);
        Ok(Some(FileNodeId {
            id: hasher.finish() as u128,
        }))
    }
}
