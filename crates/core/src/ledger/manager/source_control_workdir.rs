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

use super::projection_cleanup::drop_unanchored_projection_path;
use super::source_control_workdir_helpers::{
    discard_added, discard_tracked_add, restore_doc_projection_at_path, workspace_path_exists,
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
        let pending = self.run_on_local_repo(repo_name, |db| pending_fs::get(db, &normalized))?;
        if let Some(entry) = pending {
            return self.discard_pending_entry(repo_name, normalized, entry);
        }

        let staged =
            self.run_on_local_repo(repo_name, |db| staging::get_staged(db, &normalized))?;
        if let Some(entry) = staged {
            return self.discard_staged_entry(repo_name, normalized, entry);
        }

        anyhow::bail!("Path is not in pending_fs_ops or staging: {}", normalized);
    }

    fn discard_pending_entry(
        &self,
        repo_name: &str,
        normalized: String,
        entry: pending_fs::PendingFsEntry,
    ) -> Result<()> {
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

    fn discard_staged_entry(
        &self,
        repo_name: &str,
        normalized: String,
        entry: staging::StagedEntry,
    ) -> Result<()> {
        match entry.status {
            ChangeStatus::Added => match entry.doc_id {
                Some(doc_id) => discard_staged_tracked_add(self, repo_name, &normalized, doc_id)?,
                None => discard_staged_added(self, repo_name, &normalized)?,
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
                    discard_staged_added(self, repo_name, &normalized)?;
                }
                restore_doc_projection_at_path(self, repo_name, doc_id, &canonical_path)?;
                clear_staged_for_doc_or_path(self, repo_name, Some(doc_id), &normalized)?;
            }
        }

        Ok(())
    }
}

fn discard_staged_added(repo: &RepoManager, repo_name: &str, path: &str) -> Result<()> {
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    if workspace_path_exists(
        &file_path,
        &format!(
            "Failed to stat workspace path while discarding staged added file {}",
            path
        ),
    )? {
        repo.record_projection_delete(repo_name, path);
        if let Err(err) = std::fs::remove_file(&file_path) {
            repo.clear_projection_guard(repo_name, path);
            return Err(err.into());
        }
    }
    repo.run_on_local_repo(repo_name, |db| {
        let _ = staging::take_staged(db, path)?;
        drop_unanchored_projection_path(db, path)?;
        Ok(())
    })
}

fn discard_staged_tracked_add(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
    doc_id: DocId,
) -> Result<()> {
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    if workspace_path_exists(
        &file_path,
        &format!(
            "Failed to stat workspace path while discarding staged tracked add {}",
            path
        ),
    )? {
        repo.record_projection_delete(repo_name, path);
        if let Err(err) = std::fs::remove_file(&file_path) {
            repo.clear_projection_guard(repo_name, path);
            return Err(err.into());
        }
    }
    let canonical_path = repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
        .map(|meta| meta.path)
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", doc_id))?;
    restore_doc_projection_at_path(repo, repo_name, doc_id, &canonical_path)?;
    clear_staged_for_doc_or_path(repo, repo_name, Some(doc_id), path)
}

fn clear_staged_for_doc_or_path(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: Option<DocId>,
    path: &str,
) -> Result<()> {
    repo.run_on_local_repo(repo_name, |db| {
        if let Some(doc_id) = doc_id {
            let staged_paths = staging::list_staged_entries_for_doc(db, doc_id)?
                .into_iter()
                .map(|(path, _)| path)
                .collect::<Vec<_>>();
            for staged_path in staged_paths {
                let _ = staging::take_staged(db, &staged_path)?;
            }
            return Ok(());
        }
        let _ = staging::take_staged(db, path)?;
        Ok(())
    })
}
