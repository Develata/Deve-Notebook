//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/projection#projection-contract
//!
use crate::ledger::RepoManager;
use crate::ledger::reconcile;
use anyhow::{Context, Result};

impl RepoManager {
    pub(super) fn apply_external_file_ops_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
        doc_id_hint: Option<crate::models::DocId>,
    ) -> Result<()> {
        self.apply_file_ops_in_local_repo(
            repo_name,
            normalized_path,
            doc_id_hint,
            "external_apply_to_ledger",
        )
    }

    fn apply_file_ops_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
        doc_id_hint: Option<crate::models::DocId>,
        source: &str,
    ) -> Result<()> {
        let (doc_id, _ops) = self.apply_file_structure_in_local_repo(
            repo_name,
            normalized_path,
            doc_id_hint,
            source,
        )?;
        let disk_path = self.local_repo_workspace_path(repo_name, normalized_path)?;
        let disk_content = std::fs::read_to_string(&disk_path).with_context(|| {
            format!(
                "Failed to read staged workspace file {} at {:?}",
                normalized_path, disk_path
            )
        })?;
        let existing_ops = self.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let entries: Vec<_> = existing_ops.into_iter().map(|(_, entry)| entry).collect();
        let patch = reconcile::compute_reconcile_patch(&entries, &disk_content)?;
        reconcile::append_patch_in_local_repo(self, repo_name, doc_id, source, &patch)?;
        self.bind_workspace_inode_in_local_repo(repo_name, normalized_path, doc_id)
    }

    pub(super) fn apply_external_delete_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
        doc_id_hint: Option<crate::models::DocId>,
    ) -> Result<()> {
        self.apply_delete_in_local_repo(
            repo_name,
            normalized_path,
            doc_id_hint,
            "external_apply_to_ledger",
        )
    }

    fn apply_delete_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
        doc_id_hint: Option<crate::models::DocId>,
        source: &str,
    ) -> Result<()> {
        let Some((_doc_id, _ops)) = self.apply_file_delete_structure_in_local_repo(
            repo_name,
            normalized_path,
            doc_id_hint,
            source,
        )?
        else {
            return Ok(());
        };
        Ok(())
    }
}
