//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/projection#projection-contract
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/index#internal-path-normalization
//!
//! # Source Control 工作区辅助
//!
//! Invariants:
//! - Discard 只能恢复工作区到当前 Ledger 投影，不能改写 Ledger。

use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::{ChangeStatus, pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

use super::source_control_workdir_helpers::{
    discard_added, discard_tracked_add, restore_doc_projection_at_path,
};

impl RepoManager {
    pub(crate) fn resolve_canonical_doc_id_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<Option<DocId>> {
        self.get_tracked_docid_in_local_repo(repo_name, path)
    }

    pub(crate) fn resolve_workdir_doc_id_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<Option<DocId>> {
        if let Some(doc_id) = self.resolve_canonical_doc_id_in_local_repo(repo_name, path)? {
            return Ok(Some(doc_id));
        }
        self.run_on_local_repo(repo_name, |db| {
            if let Some(entry) = pending_fs::get(db, path)?
                && entry.doc_id.is_some()
            {
                return Ok(entry.doc_id);
            }
            if let Some(entry) = staging::get_staged(db, path)?
                && entry.doc_id.is_some()
            {
                return Ok(entry.doc_id);
            }
            Ok(None)
        })
    }

    pub(crate) fn discard_pending_workdir_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<()> {
        let normalized = to_forward_slash(path);
        let entry = self
            .run_on_local_repo(repo_name, |db| pending_fs::get(db, &normalized))?
            .ok_or_else(|| anyhow::anyhow!("Path is not in pending_fs_ops: {}", normalized))?;

        match entry.change_type {
            ChangeStatus::Added => match entry.doc_id {
                Some(doc_id) => discard_tracked_add(self, repo_name, &normalized, doc_id)?,
                None => discard_added(self, repo_name, &normalized)?,
            },
            ChangeStatus::Modified | ChangeStatus::Deleted | ChangeStatus::Renamed => {
                let doc_id = match entry.doc_id {
                    Some(doc_id) => doc_id,
                    None => self
                        .resolve_workdir_doc_id_in_local_repo(repo_name, &normalized)?
                        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", normalized))?,
                };
                let canonical_path = self
                    .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
                    .map(|meta| meta.path)
                    .ok_or_else(|| anyhow::anyhow!("Document not found: {}", doc_id))?;
                if canonical_path != normalized {
                    discard_added(self, repo_name, &normalized)?;
                }
                restore_doc_projection_at_path(self, repo_name, doc_id, &canonical_path)?;
                self.clear_pending_for_doc_in_local_repo(repo_name, doc_id, &normalized)?;
            }
        }

        Ok(())
    }
}
