//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract

use super::SyncManager;
use crate::ledger::reconcile;
use crate::models::DocId;
use crate::utils::fs::checked_exists;
use anyhow::Result;
use tracing::info;

impl SyncManager {
    pub fn reconcile_doc(&self, doc_id: DocId) -> Result<bool> {
        self.reconcile_doc_in_local_repo(self.repo.local_repo_name(), doc_id)
    }

    pub fn reconcile_doc_in_local_repo(&self, repo_name: &str, doc_id: DocId) -> Result<bool> {
        if let Some(path_str) = self
            .repo
            .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
            .map(|meta| meta.path)
        {
            let file_path = self.repo.local_repo_workspace_path(repo_name, &path_str)?;

            if checked_exists(&file_path, "workspace document path while reconciling")? {
                let disk_content = std::fs::read_to_string(&file_path)?;
                let ops = self.repo.get_local_ops_in_local_repo(repo_name, doc_id)?;

                let fix_ops = reconcile::compute_reconcile_patch(
                    &ops.into_iter().map(|(_, e)| e).collect::<Vec<_>>(),
                    &disk_content,
                )?;

                if !fix_ops.is_empty() {
                    info!(
                        "SyncManager: Applying {} fix ops for {}",
                        fix_ops.len(),
                        path_str
                    );
                    reconcile::append_patch_in_local_repo(
                        self.repo.as_ref(),
                        repo_name,
                        doc_id,
                        "local_reconcile",
                        &fix_ops,
                    )?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
