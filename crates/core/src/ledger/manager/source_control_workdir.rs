//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 04_storage#projection-contract
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#internal-path-normalization
//!
//! # Source Control 工作区辅助
//!
//! Invariants:
//! - Discard 只能恢复工作区到当前 Ledger 投影，不能改写 Ledger。
//! - 外部 Working Directory diff 的左侧永远来自当前 Ledger 投影，右侧来自磁盘文件。

use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeStatus, pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

use super::source_control_target_lookup;
use super::source_control_workdir_helpers::{
    discard_added, discard_tracked_add, rebuild_doc_projection, restore_doc_projection_at_path,
    workspace_path_exists,
};

impl RepoManager {
    pub fn workdir_diff_inputs_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<(String, String)> {
        let normalized = to_forward_slash(path);
        let old_doc_id = self.resolve_workdir_doc_id_in_local_repo(repo_name, &normalized)?;
        let old_content = match old_doc_id {
            Some(doc_id) => rebuild_doc_projection(self, repo_name, doc_id)?,
            None => String::new(),
        };
        let file_path = self.local_repo_workspace_path(repo_name, &normalized)?;
        let workspace_exists = workspace_path_exists(
            &file_path,
            &format!(
                "Failed to stat workspace path while reading workdir diff {}",
                normalized
            ),
        )?;
        let new_content = if workspace_exists {
            std::fs::read_to_string(file_path)?
        } else {
            String::new()
        };
        if old_doc_id.is_none() && !workspace_exists {
            anyhow::bail!("Doc not found: {}", normalized);
        }
        Ok((old_content, new_content))
    }

    pub fn workdir_diff_inputs_for_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<(String, String, String)> {
        let (_, path, old_content, new_content) =
            self.workdir_diff_payload_for_target_in_local_repo(repo_name, target)?;
        Ok((path, old_content, new_content))
    }

    pub fn workdir_diff_payload_for_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<(Option<DocId>, String, String, String)> {
        let path = source_control_target_lookup::resolve_change_path(self, repo_name, target)?;
        let doc_id = match target.doc_id {
            Some(doc_id) => Some(doc_id),
            None => self.resolve_workdir_doc_id_in_local_repo(repo_name, &path)?,
        };
        let (old_content, new_content) =
            self.workdir_diff_inputs_for_resolved_target(repo_name, &path, doc_id)?;
        Ok((doc_id, path, old_content, new_content))
    }

    fn workdir_diff_inputs_for_resolved_target(
        &self,
        repo_name: &str,
        path: &str,
        doc_id: Option<DocId>,
    ) -> Result<(String, String)> {
        let old_content = match doc_id {
            Some(doc_id) => rebuild_doc_projection(self, repo_name, doc_id)?,
            None => String::new(),
        };
        let file_path = self.local_repo_workspace_path(repo_name, path)?;
        let workspace_exists = workspace_path_exists(
            &file_path,
            &format!(
                "Failed to stat workspace path while reading workdir diff {}",
                path
            ),
        )?;
        let new_content = if workspace_exists {
            std::fs::read_to_string(file_path)?
        } else {
            String::new()
        };
        if doc_id.is_none() && !workspace_exists {
            anyhow::bail!("Doc not found: {}", path);
        }
        Ok((old_content, new_content))
    }

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
